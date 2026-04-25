mod bot;
mod config;
mod persistence;
mod search;
mod uptime;

use std::sync::Arc;

use clap::Parser;
use teloxide::prelude::*;
use tracing::{info, warn};

use bot::{AppState, WebhookConfig};
use config::Config;
use persistence::{DbConfig, Sound, SoundRepository};

#[derive(serde::Deserialize)]
struct DataJson {
    sounds: Vec<SoundEntry>,
}

#[derive(serde::Deserialize)]
struct SoundEntry {
    filename: String,
    text: String,
    tags: String,
}

async fn synchronize_sounds(
    data_path: &str,
    db: &SoundRepository,
) -> anyhow::Result<Vec<Sound>> {
    let data_json: DataJson = serde_json::from_str(&std::fs::read_to_string(data_path)?)?;
    info!(
        db = db.get_enabled_sounds().await?.len(),
        json = data_json.sounds.len(),
        "Synchronizing sounds"
    );

    for entry in &data_json.sounds {
        if db.find_sound_by_filename(&entry.filename).await?.is_none() {
            db.insert_sound(Sound::generate_id(), &entry.filename, &entry.text, &entry.tags)
                .await?;
        }
    }

    let db_sounds = db.get_enabled_sounds().await?;
    let mut remaining = Vec::with_capacity(db_sounds.len());
    for sound in &db_sounds {
        if data_json.sounds.iter().any(|e| e.filename == sound.filename) {
            remaining.push(sound.clone());
        } else {
            db.remove_sound(sound).await?;
        }
    }

    Ok(remaining)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::parse();

    init_tracing(&config);

    if config.admin.is_none() {
        warn!("No admin user specified. Use --admin or TELEGRAM_USER_ALIAS env var.");
    }

    info!("Starting up bot...");
    uptime::init();

    let db_config = match config.mysql_host.as_deref() {
        Some(host) => DbConfig::Mysql {
            host,
            port: &config.mysql_port,
            user: &config.mysql_user,
            password: &config.mysql_password,
            database: &config.mysql_database,
        },
        None => match config.sqlite.as_deref() {
            Some(path) => DbConfig::SqliteFile(path),
            None => DbConfig::SqliteMemory,
        },
    };

    let db = SoundRepository::connect(db_config).await?;
    let sounds = synchronize_sounds(&config.data, &db).await?;
    info!(count = sounds.len(), "Serving sounds");

    let state = Arc::new(AppState {
        db,
        sounds,
        bucket: config.bucket.clone(),
        admin: config.admin.clone(),
    });

    let webhook = config.webhook_host.map(|host| WebhookConfig {
        host,
        port: config.webhook_port,
        listen_port: config.webhook_listening_port,
        token: config.token.clone(),
    });

    let bot = Bot::new(&config.token);
    bot::run(bot, state, webhook).await;

    Ok(())
}

fn init_tracing(config: &Config) {
    use tracing_subscriber::fmt;
    use tracing_subscriber::EnvFilter;

    let filter =
        EnvFilter::try_new(&config.verbosity).unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(ref logfile) = config.logfile {
        let path = std::path::Path::new(logfile);
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        let filename = path
            .file_name()
            .unwrap_or(std::ffi::OsStr::new("rajoybot.log"));
        let file_appender = tracing_appender::rolling::never(dir, filename);
        let (writer, _guard) = tracing_appender::non_blocking(file_appender);

        // Leak the guard so the writer lives for the entire process.
        // This is intentional: main() runs until process exit, and there's
        // no clean shutdown hook we could hold the guard through.
        std::mem::forget(_guard);

        fmt().with_env_filter(filter).with_writer(writer).init();
    } else {
        fmt().with_env_filter(filter).init();
    }
}
