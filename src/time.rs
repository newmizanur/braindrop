//! Timezone-aware day boundaries.
//!
//! The database stores UTC. Everything the user sees as "today" or "this
//! week" is resolved in their configured IANA timezone first and then
//! converted back to UTC for querying.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
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

/// Parse a local wall-clock time such as `09:00` or `09:00:00`.
#[must_use]
pub fn parse_local_time(value: &str) -> Option<NaiveTime> {
    let value = value.trim();
    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .ok()
}

/// The local calendar date at `now` in `timezone`.
#[must_use]
pub fn local_date(timezone: Tz, now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&timezone).date_naive()
}

/// The local calendar day a daily notification belongs to while `now` is in
/// its delivery window.
///
/// A window that crosses midnight (for example a 23:30 digest with a two-hour
/// window) still belongs to the previous local day, so the wrap does not create
/// an early next-day notification.
#[must_use]
pub fn daily_window_date(digest_time: NaiveTime, timezone: Tz, now: DateTime<Utc>) -> NaiveDate {
    let date = local_date(timezone, now);
    if local_minutes(local_time(timezone, now)) < local_minutes(digest_time) {
        date - Duration::days(1)
    } else {
        date
    }
}

/// The local wall-clock time at `now` in `timezone`.
#[must_use]
pub fn local_time(timezone: Tz, now: DateTime<Utc>) -> NaiveTime {
    now.with_timezone(&timezone).time()
}

/// Minutes since local midnight, used to compare against a digest time without
/// constructing a possibly non-existent local datetime during a DST jump.
#[must_use]
pub fn local_minutes(time: NaiveTime) -> i64 {
    i64::from(time.hour()) * 60 + i64::from(time.minute())
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
    fn parses_local_times_and_rejects_bad_values() {
        assert_eq!(
            parse_local_time("09:30").unwrap(),
            NaiveTime::from_hms_opt(9, 30, 0).unwrap()
        );
        assert_eq!(
            parse_local_time("23:05:59").unwrap(),
            NaiveTime::from_hms_opt(23, 5, 59).unwrap()
        );
        assert!(parse_local_time("25:00").is_none());
        assert!(parse_local_time("morning").is_none());
    }

    #[test]
    fn local_date_and_time_follow_the_timezone() {
        let tokyo = parse_timezone("Asia/Tokyo").unwrap();
        let now = DateTime::parse_from_rfc3339("2026-11-19T00:30:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            local_date(tokyo, now),
            NaiveDate::from_ymd_opt(2026, 11, 19).unwrap()
        );
        assert_eq!(
            local_time(tokyo, now),
            NaiveTime::from_hms_opt(9, 30, 0).unwrap()
        );
        assert_eq!(local_minutes(local_time(tokyo, now)), 9 * 60 + 30);
    }

    #[test]
    fn daily_window_date_keeps_a_cross_midnight_window_on_one_day() {
        let utc = chrono_tz::UTC;
        let late = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
        let before = DateTime::parse_from_rfc3339("2026-11-19T23:30:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        let after = DateTime::parse_from_rfc3339("2026-11-20T00:15:00+00:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(
            daily_window_date(late, utc, before),
            NaiveDate::from_ymd_opt(2026, 11, 19).unwrap()
        );
        assert_eq!(
            daily_window_date(late, utc, after),
            NaiveDate::from_ymd_opt(2026, 11, 19).unwrap()
        );
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
