use rand::Rng;
use sqlx::any::AnyRow;
use sqlx::{AnyPool, Row};
use tracing::{debug, info};

pub mod tools;

// --- Models ---

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Sound {
    pub id: i64,
    pub filename: String,
    pub text: String,
    pub tags: String,
    pub disabled: bool,
}

impl Sound {
    pub(crate) fn from_row(row: &AnyRow) -> Self {
        Self {
            id: row.get("id"),
            filename: row.get("filename"),
            text: row.get("text"),
            tags: row.get("tags"),
            disabled: row.get("disabled"),
        }
    }

    pub fn generate_id() -> i64 {
        rand::rng().random_range(10_000_000..99_999_999)
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
}

impl User {
    fn from_row(row: &AnyRow) -> Self {
        Self {
            id: row.get("id"),
            is_bot: row.get("is_bot"),
            first_name: row.get("first_name"),
            last_name: row.get("last_name"),
            username: row.get("username"),
            language_code: row.get("language_code"),
        }
    }
}

impl From<&teloxide::types::User> for User {
    fn from(u: &teloxide::types::User) -> Self {
        Self {
            id: u.id.0 as i64,
            is_bot: u.is_bot,
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
            username: u.username.clone(),
            language_code: u.language_code.clone(),
        }
    }
}

// --- Database configuration ---

pub enum DbConfig<'a> {
    Mysql {
        host: &'a str,
        port: &'a str,
        user: &'a str,
        password: &'a str,
        database: &'a str,
    },
    SqliteFile(&'a str),
    SqliteMemory,
}

impl DbConfig<'_> {
    fn to_url(&self) -> String {
        match self {
            Self::Mysql { host, port, user, password, database } => {
                info!(host, port, "Using MySQL as persistence layer");
                format!("mysql://{user}:{password}@{host}:{port}/{database}")
            }
            Self::SqliteFile(path) => {
                info!(path, "Using SQLite as persistence layer");
                format!("sqlite://{path}?mode=rwc")
            }
            Self::SqliteMemory => {
                info!("Using in-memory SQLite as persistence layer");
                "sqlite://:memory:".into()
            }
        }
    }
}

// --- Repository ---

pub struct SoundRepository {
    pool: AnyPool,
    is_mysql: bool,
}

impl SoundRepository {
    pub async fn connect(config: DbConfig<'_>) -> Result<Self, sqlx::Error> {
        sqlx::any::install_default_drivers();
        let url = config.to_url();
        let is_mysql = matches!(config, DbConfig::Mysql { .. });
        let pool = AnyPool::connect(&url).await?;
        let repo = Self { pool, is_mysql };
        repo.create_tables().await?;
        Ok(repo)
    }

    pub(crate) fn pool(&self) -> &AnyPool {
        &self.pool
    }

    // --- Schema ---

    async fn create_tables(&self) -> Result<(), sqlx::Error> {
        if self.is_mysql {
            self.create_tables_mysql().await
        } else {
            self.create_tables_sqlite().await
        }
    }

    async fn create_tables_mysql(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sound (
                id BIGINT PRIMARY KEY,
                filename VARCHAR(255) NOT NULL UNIQUE,
                text VARCHAR(512) NOT NULL,
                tags VARCHAR(512) NOT NULL,
                disabled BOOLEAN NOT NULL DEFAULT FALSE,
                INDEX idx_sound_filename (filename)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user (
                id BIGINT PRIMARY KEY,
                is_bot BOOLEAN NOT NULL,
                first_name VARCHAR(255) NOT NULL,
                last_name VARCHAR(255),
                username VARCHAR(255),
                language_code VARCHAR(16),
                first_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS queryhistory (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                text VARCHAR(512),
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES user(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS resulthistory (
                id BIGINT AUTO_INCREMENT PRIMARY KEY,
                user_id BIGINT NOT NULL,
                sound_id BIGINT NOT NULL,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES user(id),
                FOREIGN KEY (sound_id) REFERENCES sound(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_tables_sqlite(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sound (
                id INTEGER PRIMARY KEY,
                filename TEXT NOT NULL UNIQUE,
                text TEXT NOT NULL,
                tags TEXT NOT NULL,
                disabled INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sound_filename ON sound(filename)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user (
                id INTEGER PRIMARY KEY,
                is_bot INTEGER NOT NULL,
                first_name TEXT NOT NULL,
                last_name TEXT,
                username TEXT,
                language_code TEXT,
                first_seen DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS queryhistory (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                text TEXT,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES user(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS resulthistory (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                sound_id INTEGER NOT NULL,
                timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES user(id),
                FOREIGN KEY (sound_id) REFERENCES sound(id)
            )",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // --- Sounds ---

    pub async fn get_enabled_sounds(&self) -> Result<Vec<Sound>, sqlx::Error> {
        sqlx::query("SELECT id, filename, text, tags, disabled FROM sound WHERE disabled = 0")
            .fetch_all(&self.pool)
            .await
            .map(|rows| rows.iter().map(Sound::from_row).collect())
    }

    pub async fn find_sound_by_filename(&self, filename: &str) -> Result<Option<Sound>, sqlx::Error> {
        sqlx::query("SELECT id, filename, text, tags, disabled FROM sound WHERE filename = ?")
            .bind(filename)
            .fetch_optional(&self.pool)
            .await
            .map(|opt| opt.as_ref().map(Sound::from_row))
    }

    pub async fn insert_sound(&self, id: i64, filename: &str, text: &str, tags: &str) -> Result<(), sqlx::Error> {
        info!(id, filename, "Adding sound");
        sqlx::query("INSERT INTO sound (id, filename, text, tags, disabled) VALUES (?, ?, ?, ?, 0)")
            .bind(id)
            .bind(filename)
            .bind(text)
            .bind(tags)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Soft-deletes a sound if it has usage history, hard-deletes otherwise.
    pub async fn remove_sound(&self, sound: &Sound) -> Result<(), sqlx::Error> {
        info!(sound.id, sound.filename, "Removing sound");
        let has_usage: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM resulthistory WHERE sound_id = ?")
                .bind(sound.id)
                .fetch_one(&self.pool)
                .await?;

        if has_usage > 0 {
            sqlx::query("UPDATE sound SET disabled = 1 WHERE id = ?")
                .bind(sound.id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("DELETE FROM sound WHERE id = ?")
                .bind(sound.id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    // --- Users ---

    pub async fn upsert_user(&self, user: &User) -> Result<(), sqlx::Error> {
        let exists = self.find_user(user.id).await?.is_some();
        if exists {
            debug!(user.id, "Updating user");
            sqlx::query(
                "UPDATE user SET is_bot = ?, first_name = ?, last_name = ?, username = ?, language_code = ? WHERE id = ?",
            )
            .bind(user.is_bot)
            .bind(&user.first_name)
            .bind(&user.last_name)
            .bind(&user.username)
            .bind(&user.language_code)
            .bind(user.id)
            .execute(&self.pool)
            .await?;
        } else {
            info!(user.id, user.first_name, "Adding user");
            sqlx::query(
                "INSERT INTO user (id, is_bot, first_name, last_name, username, language_code) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(user.id)
            .bind(user.is_bot)
            .bind(&user.first_name)
            .bind(&user.last_name)
            .bind(&user.username)
            .bind(&user.language_code)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    pub async fn find_user(&self, id: i64) -> Result<Option<User>, sqlx::Error> {
        sqlx::query("SELECT id, is_bot, first_name, last_name, username, language_code FROM user WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map(|opt| opt.as_ref().map(User::from_row))
    }

    pub async fn user_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM user")
            .fetch_one(&self.pool)
            .await
    }

    // --- History ---

    pub async fn record_query(&self, user: &teloxide::types::User, text: &str) -> Result<(), sqlx::Error> {
        self.upsert_user(&User::from(user)).await?;
        sqlx::query("INSERT INTO queryhistory (user_id, text) VALUES (?, ?)")
            .bind(user.id.0 as i64)
            .bind(text)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_result(&self, user: &teloxide::types::User, sound_id: i64) -> Result<(), sqlx::Error> {
        self.upsert_user(&User::from(user)).await?;
        sqlx::query("INSERT INTO resulthistory (user_id, sound_id) VALUES (?, ?)")
            .bind(user.id.0 as i64)
            .bind(sound_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn query_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM queryhistory")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn result_count(&self) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar("SELECT COUNT(*) FROM resulthistory")
            .fetch_one(&self.pool)
            .await
    }
}
