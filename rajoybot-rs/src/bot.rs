use std::sync::Arc;

use teloxide::utils::command::BotCommands;
use teloxide::payloads::AnswerInlineQuerySetters;
use teloxide::prelude::*;
use teloxide::types::{
    ChosenInlineResult, InlineQuery, InlineQueryResult, InlineQueryResultVoice, Me, ParseMode,
};
use tracing::{debug, error, info};

use crate::persistence::tools::latest_sounds_for_user;
use crate::persistence::{Sound, SoundRepository, User};
use crate::search::{preprocess_query, search_sounds};
use crate::uptime;

const MAX_INLINE_RESULTS: usize = 48;
const RECENT_SOUNDS_LIMIT: usize = 3;

/// Shared application state injected into all handlers via `dptree::deps!`.
pub struct AppState {
    pub db: SoundRepository,
    pub sounds: Vec<Sound>,
    pub bucket: String,
    pub admin: Option<String>,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    /// Welcome message explaining the bot is inline-only
    Start,
    /// Show bot statistics (admin only)
    Stats,
    /// Show machine and bot uptime (admin only)
    Uptime,
}

fn make_voice_result(sound: &Sound, bucket: &str, title_override: Option<&str>) -> InlineQueryResult {
    let url = url::Url::parse(&format!("{bucket}{}", sound.filename)).expect("invalid sound URL");
    InlineQueryResult::Voice(
        InlineQueryResultVoice::new(sound.id.to_string(), url, title_override.unwrap_or(&sound.text))
            .caption(sound.text.clone()),
    )
}

/// Webhook configuration, if the bot should use webhooks instead of polling.
pub struct WebhookConfig {
    pub host: String,
    pub port: u16,
    pub listen_port: u16,
    pub token: String,
}

pub async fn run(bot: Bot, state: Arc<AppState>, webhook: Option<WebhookConfig>) {
    let inline_handler = Update::filter_inline_query()
        .branch(
            dptree::filter(|q: InlineQuery| q.query.is_empty()).endpoint(handle_empty_query),
        )
        .branch(dptree::endpoint(handle_text_query));

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(handle_command))
        .branch(inline_handler)
        .branch(Update::filter_chosen_inline_result().endpoint(handle_chosen_result));

    if let Some(admin) = &state.admin {
        info!(admin, "Admin commands enabled");
    }

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build();

    match webhook {
        Some(wh) => {
            let url = format!("https://{}:{}/{}/", wh.host, wh.port, wh.token)
                .parse()
                .expect("invalid webhook URL");
            info!(host = wh.host, port = wh.port, "Starting webhook");

            let listener = teloxide::update_listeners::webhooks::axum(
                bot,
                teloxide::update_listeners::webhooks::Options::new(
                    ([0, 0, 0, 0], wh.listen_port).into(),
                    url,
                ),
            )
            .await
            .expect("failed to create webhook listener");

            dispatcher
                .dispatch_with_listener(listener, LoggingErrorHandler::new())
                .await;
        }
        None => {
            info!("Starting polling");
            dispatcher.dispatch().await;
        }
    }
}

// --- Command handlers ---

async fn handle_command(bot: Bot, msg: Message, me: Me, state: Arc<AppState>) -> ResponseResult<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };

    let Ok(cmd) = Command::parse(text, me.username()) else {
        return Ok(());
    };

    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "Este bot es inline. Teclea su nombre en una conversaci\u{f3}n/grupo y podras enviar un mensaje moderno.",
            )
            .await?;

            if let Some(from) = &msg.from {
                if let Err(e) = state.db.upsert_user(&User::from(from)).await {
                    error!("Failed to save user: {e}");
                }
            }
        }
        Command::Stats => {
            if !is_admin(&msg, &state) {
                return Ok(());
            }
            let users = state.db.user_count().await.unwrap_or(0);
            let queries = state.db.query_count().await.unwrap_or(0);
            let results = state.db.result_count().await.unwrap_or(0);

            #[allow(deprecated)]
            let mode = ParseMode::Markdown;
            bot.send_message(
                msg.chat.id,
                format!(
                    "\u{1f916} {}\n*All time stats:*\n\u{1f465} Users: {users}\n\u{1f50e} Queries: {queries}\n\u{1f50a} Results: {results}",
                    uptime::bot_uptime(),
                ),
            )
            .parse_mode(mode)
            .await?;
        }
        Command::Uptime => {
            if !is_admin(&msg, &state) {
                return Ok(());
            }
            bot.send_message(
                msg.chat.id,
                format!(
                    "\u{1f4bb} {}\n\u{231b} {}\n\u{1f916} {}",
                    uptime::machine_info(),
                    uptime::machine_uptime(),
                    uptime::bot_uptime(),
                ),
            )
            .await?;
        }
    }

    Ok(())
}

fn is_admin(msg: &Message, state: &AppState) -> bool {
    let Some(admin) = &state.admin else {
        return false;
    };
    msg.from
        .as_ref()
        .and_then(|u| u.username.as_ref())
        .is_some_and(|username| username == admin)
}

// --- Inline query handlers ---

async fn handle_empty_query(
    bot: Bot,
    q: InlineQuery,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    debug!(user_id = q.from.id.0, "Empty inline query");

    let recent = latest_sounds_for_user(&state.db, q.from.id.0 as i64, RECENT_SOUNDS_LIMIT)
        .await
        .unwrap_or_default();

    let recent_ids: std::collections::HashSet<i64> = recent.iter().map(|s| s.id).collect();

    let mut results: Vec<InlineQueryResult> = recent
        .iter()
        .map(|s| {
            let title = format!("\u{1f55a} {}", s.text);
            make_voice_result(s, &state.bucket, Some(&title))
        })
        .collect();

    results.extend(
        state
            .sounds
            .iter()
            .filter(|s| !recent_ids.contains(&s.id))
            .take(MAX_INLINE_RESULTS - results.len())
            .map(|s| make_voice_result(s, &state.bucket, None)),
    );

    bot.answer_inline_query(&q.id, results)
        .is_personal(true)
        .cache_time(5)
        .await?;

    if let Err(e) = state.db.record_query(&q.from, &q.query).await {
        error!("Couldn't save query: {e}");
    }

    Ok(())
}

async fn handle_text_query(
    bot: Bot,
    q: InlineQuery,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    let preprocessed = preprocess_query(&q.query);
    debug!(user_id = q.from.id.0, raw = q.query, preprocessed, "Text inline query");

    let results: Vec<InlineQueryResult> = search_sounds(&preprocessed, &state.sounds)
        .into_iter()
        .take(MAX_INLINE_RESULTS)
        .map(|s| make_voice_result(s, &state.bucket, None))
        .collect();

    bot.answer_inline_query(&q.id, results)
        .cache_time(5)
        .await?;

    if let Err(e) = state.db.record_query(&q.from, &q.query).await {
        error!("Couldn't save query: {e}");
    }

    Ok(())
}

async fn handle_chosen_result(
    _bot: Bot,
    chosen: ChosenInlineResult,
    state: Arc<AppState>,
) -> ResponseResult<()> {
    debug!(result_id = chosen.result_id, user_id = chosen.from.id.0, "Chosen inline result");

    if let Ok(sound_id) = chosen.result_id.parse::<i64>() {
        if let Err(e) = state.db.record_result(&chosen.from, sound_id).await {
            error!("Couldn't save result: {e}");
        }
    }

    Ok(())
}
