//! Formatting helpers shared by the views.

use std::time::Duration;

/// Format a duration as `m:ss` or `h:mm:ss`.
pub fn duration(value: Duration) -> String {
    let total_seconds = value.as_secs();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3600;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// Format a number with narrow spaces as thousands separators.
pub fn count(value: u64) -> String {
    let raw = value.to_string();
    let mut result = String::with_capacity(raw.len() + raw.len() / 3);
    for (index, ch) in raw.chars().enumerate() {
        if index > 0 && (raw.len() - index).is_multiple_of(3) {
            result.push('\u{2009}');
        }
        result.push(ch);
    }
    result
}

/// Turn an ISO `YYYY-MM-DD...` date into `DD.MM.YYYY`.
pub fn date(value: &str) -> String {
    let date_part = value.get(..10).unwrap_or(value);
    let mut parts = date_part.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day)) if year.len() == 4 => {
            format!("{day}.{month}.{year}")
        }
        _ => value.to_string(),
    }
}

/// A coarse Russian plural helper: `1 трек`, `2 трека`, `5 треков`.
pub fn plural(n: u64, one: &str, few: &str, many: &str) -> String {
    let last_two = n % 100;
    let last = n % 10;
    let word = if (11..=14).contains(&last_two) {
        many
    } else if last == 1 {
        one
    } else if (2..=4).contains(&last) {
        few
    } else {
        many
    };
    format!("{} {word}", count(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations() {
        assert_eq!(duration(Duration::from_secs(0)), "0:00");
        assert_eq!(duration(Duration::from_secs(65)), "1:05");
        assert_eq!(duration(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn formats_dates() {
        assert_eq!(date("2018-04-01T12:00:00"), "01.04.2018");
        assert_eq!(date("unknown"), "unknown");
    }

    #[test]
    fn pluralizes_ru() {
        assert_eq!(plural(1, "трек", "трека", "треков"), "1 трек");
        assert_eq!(plural(3, "трек", "трека", "треков"), "3 трека");
        assert_eq!(plural(11, "трек", "трека", "треков"), "11 треков");
        assert_eq!(plural(248, "трек", "трека", "треков"), "248 треков");
    }
}
