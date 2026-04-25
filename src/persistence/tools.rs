use std::collections::HashSet;

use tracing::debug;

use super::{Sound, SoundRepository};

/// Returns the most recently used (non-disabled) sounds for a user, deduplicated.
pub async fn latest_sounds_for_user(
    repo: &SoundRepository,
    user_id: i64,
    limit: usize,
) -> Result<Vec<Sound>, sqlx::Error> {
    let fetch_limit = (limit * 3) as i64;
    let rows = sqlx::query(
        "SELECT s.id, s.filename, s.text, s.tags, s.disabled
         FROM resulthistory r
         JOIN sound s ON r.sound_id = s.id
         WHERE r.user_id = ? AND s.disabled = 0
         ORDER BY r.timestamp DESC
         LIMIT ?",
    )
    .bind(user_id)
    .bind(fetch_limit)
    .fetch_all(repo.pool())
    .await?;

    let mut seen = HashSet::new();
    let sounds: Vec<Sound> = rows
        .iter()
        .map(Sound::from_row)
        .filter(|s| seen.insert(s.id))
        .take(limit)
        .collect();

    debug!(count = sounds.len(), user_id, "Fetched recent sounds");
    Ok(sounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::User;

    async fn setup() -> (SoundRepository, teloxide::types::User) {
        let repo = SoundRepository::test_repo().await;
        for (id, name) in [(1, "cuanto_peor.ogg"), (2, "viva_vino.ogg"), (3, "cataluna.ogg")] {
            repo.insert_sound(id, name, name, name).await.unwrap();
        }
        let tg_user = teloxide::types::User {
            id: teloxide::types::UserId(42),
            is_bot: false,
            first_name: "Mariano".into(),
            last_name: None,
            username: None,
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        };
        let user = User::from(&tg_user);
        repo.upsert_user(&user).await.unwrap();
        (repo, tg_user)
    }

    #[tokio::test]
    async fn returns_recent_sounds() {
        let (repo, tg_user) = setup().await;
        repo.record_result(&tg_user, 1).await.unwrap();
        repo.record_result(&tg_user, 2).await.unwrap();

        let recent = latest_sounds_for_user(&repo, 42, 3).await.unwrap();
        assert_eq!(recent.len(), 2);
        let ids: Vec<i64> = recent.iter().map(|s| s.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[tokio::test]
    async fn deduplicates_results() {
        let (repo, tg_user) = setup().await;
        repo.record_result(&tg_user, 1).await.unwrap();
        repo.record_result(&tg_user, 1).await.unwrap();
        repo.record_result(&tg_user, 1).await.unwrap();

        let recent = latest_sounds_for_user(&repo, 42, 3).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, 1);
    }

    #[tokio::test]
    async fn respects_limit() {
        let (repo, tg_user) = setup().await;
        repo.record_result(&tg_user, 1).await.unwrap();
        repo.record_result(&tg_user, 2).await.unwrap();
        repo.record_result(&tg_user, 3).await.unwrap();

        let recent = latest_sounds_for_user(&repo, 42, 2).await.unwrap();
        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn excludes_disabled_sounds() {
        let (repo, tg_user) = setup().await;
        repo.record_result(&tg_user, 1).await.unwrap();

        // Disable sound 1
        sqlx::query("UPDATE sound SET disabled = 1 WHERE id = 1")
            .execute(repo.pool())
            .await
            .unwrap();

        let recent = latest_sounds_for_user(&repo, 42, 3).await.unwrap();
        assert!(recent.is_empty());
    }

    #[tokio::test]
    async fn empty_for_unknown_user() {
        let (repo, _) = setup().await;
        let recent = latest_sounds_for_user(&repo, 999, 3).await.unwrap();
        assert!(recent.is_empty());
    }
}
