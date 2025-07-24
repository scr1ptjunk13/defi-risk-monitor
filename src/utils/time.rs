use chrono::{DateTime, Utc, Duration, NaiveDateTime};
use std::collections::HashMap;

/// Get current UTC timestamp
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// Convert timestamp to human readable format
pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Calculate time difference in seconds
pub fn time_diff_seconds(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    (end - start).num_seconds()
}

/// Calculate time difference in minutes
pub fn time_diff_minutes(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    (end - start).num_minutes()
}

/// Calculate time difference in hours
pub fn time_diff_hours(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    (end - start).num_hours()
}

/// Calculate time difference in days
pub fn time_diff_days(start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    (end - start).num_days()
}

/// Get start of day for a given timestamp
pub fn start_of_day(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    timestamp.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
}

/// Get end of day for a given timestamp
pub fn end_of_day(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    timestamp.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc()
}

/// Get timestamp N days ago
pub fn days_ago(days: i64) -> DateTime<Utc> {
    Utc::now() - Duration::days(days)
}

/// Get timestamp N hours ago
pub fn hours_ago(hours: i64) -> DateTime<Utc> {
    Utc::now() - Duration::hours(hours)
}

/// Get timestamp N minutes ago
pub fn minutes_ago(minutes: i64) -> DateTime<Utc> {
    Utc::now() - Duration::minutes(minutes)
}

/// Check if timestamp is within the last N minutes
pub fn is_within_last_minutes(timestamp: DateTime<Utc>, minutes: i64) -> bool {
    let threshold = minutes_ago(minutes);
    timestamp >= threshold
}

/// Check if timestamp is within the last N hours
pub fn is_within_last_hours(timestamp: DateTime<Utc>, hours: i64) -> bool {
    let threshold = hours_ago(hours);
    timestamp >= threshold
}

/// Check if timestamp is within the last N days
pub fn is_within_last_days(timestamp: DateTime<Utc>, days: i64) -> bool {
    let threshold = days_ago(days);
    timestamp >= threshold
}

/// Group timestamps by day
pub fn group_by_day(timestamps: Vec<DateTime<Utc>>) -> HashMap<String, Vec<DateTime<Utc>>> {
    let mut grouped = HashMap::new();
    
    for timestamp in timestamps {
        let day_key = timestamp.format("%Y-%m-%d").to_string();
        grouped.entry(day_key).or_insert_with(Vec::new).push(timestamp);
    }
    
    grouped
}

/// Group timestamps by hour
pub fn group_by_hour(timestamps: Vec<DateTime<Utc>>) -> HashMap<String, Vec<DateTime<Utc>>> {
    let mut grouped = HashMap::new();
    
    for timestamp in timestamps {
        let hour_key = timestamp.format("%Y-%m-%d %H:00").to_string();
        grouped.entry(hour_key).or_insert_with(Vec::new).push(timestamp);
    }
    
    grouped
}

/// Parse ISO 8601 timestamp string
pub fn parse_iso_timestamp(timestamp_str: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(timestamp_str).map(|dt| dt.with_timezone(&Utc))
}

/// Convert Unix timestamp to DateTime<Utc>
pub fn from_unix_timestamp(timestamp: i64) -> Option<DateTime<Utc>> {
    NaiveDateTime::from_timestamp_opt(timestamp, 0).map(|dt| dt.and_utc())
}

/// Convert DateTime<Utc> to Unix timestamp
pub fn to_unix_timestamp(datetime: DateTime<Utc>) -> i64 {
    datetime.timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_time_diff_seconds() {
        let start = Utc::now();
        let end = start + Duration::seconds(30);
        assert_eq!(time_diff_seconds(start, end), 30);
    }
    
    #[test]
    fn test_days_ago() {
        let past = days_ago(7);
        let now = Utc::now();
        assert!(time_diff_days(past, now) >= 6); // Allow for small timing differences
    }
    
    #[test]
    fn test_is_within_last_hours() {
        let recent = hours_ago(1);
        assert!(is_within_last_hours(recent, 2));
        
        let old = hours_ago(5);
        assert!(!is_within_last_hours(old, 2));
    }
}
