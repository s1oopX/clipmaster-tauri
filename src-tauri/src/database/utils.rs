use chrono::{TimeZone, Utc};
use chrono_tz::Tz;

use crate::settings::DEFAULT_TIME_ZONE;

pub(super) fn preview_from_content(content: &str) -> String {
    let char_count = content.chars().count();
    if char_count > 100 {
        let preview_text: String = content.chars().take(100).collect();
        format!("{}...", preview_text)
    } else {
        content.to_string()
    }
}

pub fn date_key_now(time_zone: &str) -> String {
    date_key_from_timestamp(Utc::now().timestamp_millis(), time_zone)
}

pub(super) fn date_key_from_timestamp(timestamp: i64, time_zone: &str) -> String {
    let tz = parse_time_zone(time_zone);

    Utc.timestamp_millis_opt(timestamp)
        .single()
        .unwrap_or_else(Utc::now)
        .with_timezone(&tz)
        .format("%Y-%m-%d")
        .to_string()
}

fn parse_time_zone(time_zone: &str) -> Tz {
    time_zone
        .parse::<Tz>()
        .ok()
        .or_else(|| DEFAULT_TIME_ZONE.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::Asia::Shanghai)
}
