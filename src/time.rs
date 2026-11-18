//! Timezone-aware day boundaries.
//!
//! The database stores UTC. Everything the user sees as "today" or "this
//! week" is resolved in their configured IANA timezone first and then
//! converted back to UTC for querying.

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

/// Parse an IANA timezone name such as `Europe/Berlin`.
#[must_use]
pub fn parse_timezone(name: &str) -> Option<Tz> {
    name.parse::<Tz>().ok()
}

/// Fall back to UTC rather than failing a request because a stored preference
/// is no longer a valid timezone.
#[must_use]
pub fn timezone_or_utc(name: &str) -> Tz {
    parse_timezone(name).unwrap_or(Tz::UTC)
}

/// The UTC instant at which `date` begins in `timezone`.
#[must_use]
pub fn start_of_day(timezone: Tz, date: NaiveDate) -> DateTime<Utc> {
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .expect("midnight is always a valid time");
    // Some zones skip midnight when DST starts. Walk forward until a local
    // time exists; the first valid instant is the real start of the day.
    for hours in 0..=3 {
        let candidate = midnight + Duration::hours(hours);
        if let Some(local) = timezone.from_local_datetime(&candidate).earliest() {
            return local.with_timezone(&Utc);
        }
    }
    Utc.from_utc_datetime(&midnight)
}

/// Half-open `[start, end)` bounds for a calendar day in `timezone`.
#[must_use]
pub fn day_bounds(timezone: Tz, date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    let next = date + Duration::days(1);
    (start_of_day(timezone, date), start_of_day(timezone, next))
}

/// The first day of the week containing `date`.
///
/// `week_start` uses JavaScript's convention: 0 = Sunday, 1 = Monday, ... 6 =
/// Saturday.
#[must_use]
pub fn start_of_week(date: NaiveDate, week_start: i64) -> NaiveDate {
    let weekday = i64::from(date.weekday().num_days_from_sunday());
    let offset = (weekday - week_start).rem_euclid(7);
    date - Duration::days(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_bounds_respect_the_timezone() {
        let berlin = parse_timezone("Europe/Berlin").unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let (start, end) = day_bounds(berlin, date);
        assert_eq!(start.to_rfc3339(), "2026-06-30T22:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-07-01T22:00:00+00:00");
    }

    #[test]
    fn day_bounds_handle_a_skipped_midnight() {
        let santiago = parse_timezone("America/Santiago").unwrap();
        let date = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
        let (start, end) = day_bounds(santiago, date);
        // Chile jumps from 00:00 to 01:00, so the day starts at 04:00 UTC.
        assert_eq!(start.to_rfc3339(), "2026-09-06T04:00:00+00:00");
        assert_eq!(end.to_rfc3339(), "2026-09-07T03:00:00+00:00");
    }

    #[test]
    fn week_start_handles_sunday_and_monday() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 18).unwrap(); // Wednesday
        assert_eq!(
            start_of_week(date, 1),
            NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
        );
        assert_eq!(
            start_of_week(date, 0),
            NaiveDate::from_ymd_opt(2026, 11, 15).unwrap()
        );
    }
}
