use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

/// Parse a flexible date string into a `DateTime<Utc>`.
///
/// Accepted formats (tried in order):
/// - `"today"` → today at 00:00:00 UTC
/// - `"yesterday"` → yesterday at 00:00:00 UTC
/// - `YYYY-MM-DD` → that date at 00:00:00 UTC
/// - `YYYY-MM-DDTHH:MM:SS` → that datetime in UTC
pub fn parse_since_date(input: &str) -> Result<DateTime<Utc>, String> {
    let input = input.trim();

    // Keywords
    if input.eq_ignore_ascii_case("today") {
        let today = Utc::now().date_naive();
        return Ok(today.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    if input.eq_ignore_ascii_case("yesterday") {
        let yesterday = Utc::now()
            .date_naive()
            .pred_opt()
            .unwrap_or(Utc::now().date_naive());
        return Ok(yesterday.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    // YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }

    // YYYY-MM-DDTHH:MM:SS
    if let Ok(dt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt.and_utc());
    }

    Err(format!(
        "Invalid date '{input}'. Accepted formats: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SS, 'yesterday', 'today'"
    ))
}
