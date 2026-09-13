//! Calendar validation for the native certificate clock. No fallback date.
// ------------------------=
// FUNC: unix_seconds
// DESC: Converts a valid firmware date to UTC, treating an unspecified EFI timezone as the OS UTC hardware-clock convention.
// ------------------=
pub fn unix_seconds(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    zone: i16,
) -> Option<u64> {
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let mut months = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if leap(year) {
        months[1] = 29;
    }
    if day == 0 || u64::from(day) > months[month as usize - 1] {
        return None;
    }
    let mut days = 0u64;
    for y in 1970..year {
        days += if leap(y) { 366 } else { 365 };
    }
    for length in &months[..month as usize - 1] {
        days += length;
    }
    days += u64::from(day - 1);
    let offset = if zone == 2047 {
        0
    } else if (-1440..=1440).contains(&zone) {
        zone
    } else {
        return None;
    };
    let seconds =
        days as i64 * 86400 + i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second)
            - i64::from(offset) * 60;
    if seconds <= 0 {
        None
    } else {
        Some(seconds as u64)
    }
}
// ------------------------=
// FUNC: leap
// DESC: Applies the Gregorian leap-year rule including century exceptions.
// ------------------=
fn leap(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    // ------------------------=
    // FUNC: certificate_clock_rejects_invalid_dates_and_normalizes_zones
    // DESC: Checks exact epoch values, equivalent timezones, leap-day rules and fail-closed malformed firmware values.
    // ------------------=
    fn certificate_clock_rejects_invalid_dates_and_normalizes_zones() {
        assert_eq!(unix_seconds(1970, 1, 2, 0, 0, 0, 0), Some(86400));
        assert_eq!(unix_seconds(1970, 1, 2, 1, 0, 0, 60), Some(86400));
        assert_eq!(unix_seconds(1970, 1, 2, 0, 0, 0, 2047), Some(86400));
        assert_eq!(unix_seconds(2000, 3, 1, 0, 0, 0, 0), Some(951868800));
        assert!(unix_seconds(2000, 2, 29, 0, 0, 0, 0).is_some());
        assert_eq!(unix_seconds(2100, 2, 29, 0, 0, 0, 0), None);
        assert_eq!(unix_seconds(2026, 13, 1, 0, 0, 0, 0), None);
        assert_eq!(unix_seconds(2026, 1, 0, 0, 0, 0, 0), None);
        assert_eq!(unix_seconds(2026, 1, 1, 24, 0, 0, 0), None);
        assert_eq!(unix_seconds(2026, 1, 1, 0, 60, 0, 0), None);
        assert_eq!(unix_seconds(2026, 1, 1, 0, 0, 0, 1500), None);
    }
}
