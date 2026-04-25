use std::time::Instant;

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub fn init() {
    START_TIME.get_or_init(Instant::now);
}

pub fn bot_uptime() -> String {
    let secs = START_TIME.get().map_or(0, |t| t.elapsed().as_secs());
    format!("Bot Uptime: {}", format_duration(secs))
}

pub fn machine_uptime() -> String {
    match read_proc_uptime() {
        Some(secs) => format!("Machine Uptime: {}", format_duration(secs)),
        None => "Machine Uptime: unavailable".into(),
    }
}

pub fn machine_info() -> String {
    let info = os_info::get();
    format!(
        "Running on {} {} {}",
        info.os_type(),
        info.version(),
        std::env::consts::ARCH,
    )
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h}:{m:02}:{s:02}")
}

fn read_proc_uptime() -> Option<u64> {
    std::fs::read_to_string("/proc/uptime")
        .ok()?
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .map(|v| v as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_zero() {
        assert_eq!(format_duration(0), "0:00:00");
    }

    #[test]
    fn format_seconds_only() {
        assert_eq!(format_duration(45), "0:00:45");
    }

    #[test]
    fn format_minutes_and_seconds() {
        assert_eq!(format_duration(125), "0:02:05");
    }

    #[test]
    fn format_hours_minutes_seconds() {
        assert_eq!(format_duration(3661), "1:01:01");
    }

    #[test]
    fn format_large_duration() {
        // 4 years as president... approximately
        assert_eq!(format_duration(86400), "24:00:00");
    }

    #[test]
    fn bot_uptime_after_init() {
        init();
        let output = bot_uptime();
        assert!(output.starts_with("Bot Uptime: "));
    }
}
