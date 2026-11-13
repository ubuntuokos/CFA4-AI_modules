//! Time zones for timed starts (ACS-4): a task started "at" a local time keeps that local time
//! across daylight-saving changes, and a missed start moves to its next slot in the task's own
//! IANA time zone (`Europe/Budapest`), not in UTC.
//!
//! A slot is the same local wall-clock time as the start that was missed, on the first day it is
//! still ahead. When that local time does not exist that day (the clocks jump forward over it) the
//! slot is the first moment after the gap; when it exists twice (the clocks go back), the first
//! one. The time zone database is the system's (`/usr/share/zoneinfo`), with a bundled copy as the
//! fallback; an unknown zone is an error, never a guess.

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::queue::UtcMs;

/// Longest accepted time zone name.
pub const MAX_ZONE: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    UnknownZone(String),
    /// A time outside what the calendar can represent.
    OutOfRange,
}

fn zone(name: &str) -> Result<TimeZone, ScheduleError> {
    if name.is_empty() || name.len() > MAX_ZONE {
        return Err(ScheduleError::UnknownZone(name.chars().take(MAX_ZONE).collect()));
    }
    TimeZone::get(name).map_err(|_| ScheduleError::UnknownZone(name.to_owned()))
}

fn ts(ms: UtcMs) -> Result<Timestamp, ScheduleError> {
    let ms = i64::try_from(ms).map_err(|_| ScheduleError::OutOfRange)?;
    Timestamp::from_millisecond(ms).map_err(|_| ScheduleError::OutOfRange)
}

fn ms(t: Timestamp) -> Result<UtcMs, ScheduleError> {
    UtcMs::try_from(t.as_millisecond()).map_err(|_| ScheduleError::OutOfRange)
}

/// Check that `name` is a time zone this machine knows.
pub fn check_zone(name: &str) -> Result<(), ScheduleError> {
    zone(name).map(|_| ())
}

/// The next slot after `now` for a start planned at `scheduled_at`, in time zone `name`.
pub fn next_slot(scheduled_at: UtcMs, name: &str, now: UtcMs) -> Result<UtcMs, ScheduleError> {
    let tz = zone(name)?;
    let local_time = ts(scheduled_at)?.to_zoned(tz.clone()).time();
    let now_ts = ts(now)?;
    let mut day = now_ts.to_zoned(tz.clone()).date();
    // today, or tomorrow; one more day covers a slot that falls into a gap pushed past `now`
    for _ in 0..3 {
        let candidate = day.to_datetime(local_time).to_zoned(tz.clone()).map_err(|_| ScheduleError::OutOfRange)?.timestamp();
        if candidate > now_ts {
            return ms(candidate);
        }
        day = day.tomorrow().map_err(|_| ScheduleError::OutOfRange)?;
    }
    Err(ScheduleError::OutOfRange)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: u64 = 3_600_000;

    fn at(s: &str) -> UtcMs {
        u64::try_from(s.parse::<Timestamp>().unwrap().as_millisecond()).unwrap()
    }

    #[test]
    fn a_missed_start_moves_to_the_same_local_time_on_the_next_day() {
        // planned 02:00 Budapest (01:00 UTC in winter), the machine woke at 09:00 local
        let planned = at("2026-11-12T01:00:00Z");
        assert_eq!(next_slot(planned, "Europe/Budapest", at("2026-11-12T08:00:00Z")), Ok(at("2026-11-13T01:00:00Z")));
        // woke before the slot on the same day: today
        assert_eq!(next_slot(planned, "Europe/Budapest", at("2026-11-12T00:30:00Z")), Ok(planned));
    }

    #[test]
    fn local_time_survives_daylight_saving_changes() {
        // 22:00 Budapest in summer (20:00 UTC) missed just before the October change: the next
        // slot is 22:00 winter time, 21:00 UTC, not 20:00 UTC
        let planned = at("2026-10-24T20:00:00Z");
        assert_eq!(next_slot(planned, "Europe/Budapest", at("2026-10-25T06:00:00Z")), Ok(at("2026-10-25T21:00:00Z")));
        // 02:30 does not exist on 2027-03-28 in Budapest: the first moment after the gap
        let planned = at("2027-03-27T01:30:00Z");
        let next = next_slot(planned, "Europe/Budapest", at("2027-03-27T12:00:00Z")).unwrap();
        assert_eq!(next, at("2027-03-28T01:30:00Z"), "03:30 summer time, one hour after the gap began");
        // UTC itself has no changes
        assert_eq!(next_slot(5 * H, "UTC", 30 * H), Ok(53 * H));
    }

    #[test]
    fn unknown_zones_and_hostile_times_are_errors() {
        assert!(matches!(next_slot(0, "Mars/Olympus_Mons", 1), Err(ScheduleError::UnknownZone(_))));
        assert!(matches!(next_slot(0, "", 1), Err(ScheduleError::UnknownZone(_))));
        assert!(matches!(next_slot(0, &"x".repeat(10_000), 1), Err(ScheduleError::UnknownZone(z)) if z.len() == MAX_ZONE));
        assert_eq!(next_slot(u64::MAX, "UTC", 0), Err(ScheduleError::OutOfRange));
        assert_eq!(next_slot(0, "UTC", u64::MAX), Err(ScheduleError::OutOfRange));
        assert!(check_zone("Europe/Budapest").is_ok());
    }
}
