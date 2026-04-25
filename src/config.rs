use clap::Parser;

const DEFAULT_BUCKET: &str = "https://github.com/elraro/RajoyBot/raw/master/RajoyBotSounds/";

#[derive(Parser, Debug, Clone)]
#[command(name = "rajoybot", about = "RajoyBot - Telegram bot for Rajoy audio clips")]
pub struct Config {
    /// Telegram API token given by @BotFather
    #[arg(long = "token", env = "TELEGRAM_BOT_TOKEN")]
    pub token: String,

    /// Alias of the admin user
    #[arg(long = "admin", env = "TELEGRAM_USER_ALIAS")]
    pub admin: Option<String>,

    /// Bucket or URL where audios are stored
    #[arg(short = 'b', long = "bucket", default_value = DEFAULT_BUCKET)]
    pub bucket: String,

    /// SQLite file path (in-memory if unset)
    #[arg(long = "sqlite", env = "SQLITE_FILE")]
    pub sqlite: Option<String>,

    /// MySQL host (overrides SQLite when set)
    #[arg(long = "mysql-host", env = "MYSQL_HOST")]
    pub mysql_host: Option<String>,

    /// MySQL port
    #[arg(long = "mysql-port", env = "MYSQL_PORT", default_value = "3306")]
    pub mysql_port: String,

    /// MySQL user
    #[arg(long = "mysql-user", env = "MYSQL_USER", default_value = "rajoybot")]
    pub mysql_user: String,

    /// MySQL password
    #[arg(long = "mysql-password", env = "MYSQL_PASSWORD", default_value = "rajoybot")]
    pub mysql_password: String,

    /// MySQL database name
    #[arg(long = "mysql-database", env = "MYSQL_DATABASE", default_value = "rajoybot")]
    pub mysql_database: String,

    /// Data JSON path
    #[arg(long = "data", env = "DATA_JSON", default_value = "data.json")]
    pub data: String,

    /// Log to defined file
    #[arg(long = "logfile", env = "LOGFILE")]
    pub logfile: Option<String>,

    /// Log verbosity
    #[arg(short = 'v', long = "verbosity", default_value = "info",
          value_parser = ["error", "warn", "info", "debug", "trace"])]
    pub verbosity: String,

    /// Webhook host (uses polling if unset)
    #[arg(long = "webhook-host", env = "WEBHOOK_HOST")]
    pub webhook_host: Option<String>,

    /// Webhook port (default 443)
    #[arg(long = "webhook-port", env = "WEBHOOK_PORT", default_value = "443")]
    pub webhook_port: u16,

    /// Webhook local listening IP
    #[arg(long = "webhook-listening", env = "WEBHOOK_LISTEN", default_value = "0.0.0.0")]
    pub webhook_listening: String,

    /// Webhook local listening port
    #[arg(long = "webhook-listening-port", env = "WEBHOOK_LISTEN_PORT", default_value = "8080")]
    pub webhook_listening_port: u16,
}
