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
