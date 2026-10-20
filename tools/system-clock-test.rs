#[path = "../kernel/runtime/ai/clock.rs"]
mod clock;
use clock::*;

#[test]
// ------------------------=
// FUNC: exact_requests_and_offsets
// DESC: Exercises direct speech/text requests and rejects ambiguous, quoted or invalid mutations.
// ------------------=
fn exact_requests_and_offsets() {
    for text in [b"Infinity what time is it?".as_slice(), b"Infinity, what time is it?", b"What is the current time?"] {
        assert_eq!(Request::parse(text), Some(Request::Read));
    }
    for text in [b"Set time zone to CST".as_slice(), b"set my time zone to CST", b"change the time zone to UTC minus 06:00"] {
        assert_eq!(Request::parse(text), Some(Request::SetZone(-360)));
    }
    assert_eq!(Request::parse(b"Set the time to 3:27 PM"), Some(Request::SetTime { hour: 15, minute: 27, second: 0 }));
    assert_eq!(Request::parse(b"clock set 12:01 am"), Some(Request::SetTime { hour: 0, minute: 1, second: 0 }));
    assert_eq!(Request::parse(b"clock set 12:01 pm"), Some(Request::SetTime { hour: 12, minute: 1, second: 0 }));
    assert_eq!(Request::parse(b"set time to 25:59"), Some(Request::InvalidTime));
    assert_eq!(Request::parse(b"set time to 12:61"), Some(Request::InvalidTime));
    assert_eq!(Request::parse(b"set time zone to UTC+14:01"), Some(Request::InvalidZone));
    assert_eq!(Request::parse(b"set time zone to America/Chicago"), Some(Request::InvalidZone));
    for text in [b"Do not set time to 15:00".as_slice(), b"Remember my time zone is CST", b"Explain set time to 15:00", b"say \"set time to 15:00\""] {
        assert_eq!(Request::parse(text), None);
    }
    assert_eq!(parse_zone(b"UTC+05:30"), Some(330));
    assert_eq!(parse_zone(b"UTC plus 05:30"), Some(330));
    assert_eq!(parse_zone(b"GMT-12:00"), Some(-720));
    assert_eq!(parse_zone(b"UTC+14:00"), Some(840));
    assert_eq!(parse_zone(b"UTC-14:30"), None);
}

#[test]
// ------------------------=
// FUNC: calendar_and_clock_publication
// DESC: Checks midnight/year rollover, leap dates, live correction, nanoseconds and preserved monotonic semantics.
// ------------------=
fn calendar_and_clock_publication() {
    let service = SystemClock::new();
    assert_eq!(service.utc_nanoseconds(1), None);
    service.observe(1_767_225_600, 10_000_000_000); // 2026-01-01 UTC.
    assert_eq!(service.utc_nanoseconds(11_123_456_789), Some(1_767_225_601_123_456_789));
    assert!(service.configure(Settings { offset_minutes: -360, correction_seconds: 120 }));
    assert_eq!(service.utc_nanoseconds(11_123_456_789), Some(1_767_225_721_123_456_789));
    let local = local_time(1_767_225_600, -360).unwrap();
    assert_eq!((local.year, local.month, local.day, local.hour), (2025, 12, 31, 18));
    let leap = local_time(951_782_400, 0).unwrap();
    assert_eq!((leap.year, leap.month, leap.day), (2000, 2, 29));
    assert_eq!(service.utc_nanoseconds(9_999_999_999), None);
    assert!(!service.configure(Settings { offset_minutes: 841, correction_seconds: 0 }));
    assert_eq!(service.settings().offset_minutes, -360);
    let mut out = [0; 384];
    let receipt = execute(Request::Read, Some(1_767_225_600), service.settings(), true, false, |_, _| panic!("read attempted commit"));
    assert_eq!(receipt.outcome, Outcome::Read);
    assert!(clock::format(receipt, &mut out) > 0);
}

#[test]
// ------------------------=
// FUNC: authorized_changes_commit_before_publication
// DESC: Verifies exact time changes, unchanged UTC on zone changes, failure preservation, and denied or unavailable requests.
// ------------------=
fn authorized_changes_commit_before_publication() {
    let raw = 1_767_279_600; // 2026-01-01 15:00 UTC.
    let mut committed = None;
    let zone = execute(Request::SetZone(-360), Some(raw), Settings::UTC, true, true, |settings, local| {
        assert_eq!(local.hour, 9); committed = Some(settings); true
    });
    assert_eq!(zone.outcome, Outcome::Updated);
    assert_eq!(zone.settings.utc(raw), Some(raw));
    assert_eq!(committed, Some(zone.settings));
    let time = execute(Request::SetTime { hour: 10, minute: 30, second: 15 }, Some(raw), zone.settings, true, true, |settings, local| {
        assert_eq!((local.hour, local.minute, local.second), (10, 30, 15));
        assert_eq!(settings.correction_seconds, 5_415); true
    });
    assert_eq!(time.outcome, Outcome::Updated);
    assert_eq!(time.settings.utc(raw), Some(raw + 5_415));
    let failed = execute(Request::SetZone(330), Some(raw), time.settings, true, true, |_, _| false);
    assert_eq!(failed.outcome, Outcome::SaveFailed);
    assert_eq!(failed.settings, time.settings);
    for (request, read, write, expected) in [(Request::Read, false, false, Outcome::Denied),
        (Request::SetZone(0), true, false, Outcome::Denied), (Request::InvalidTime, true, true, Outcome::InvalidTime)] {
        let denied = execute(request, Some(raw), time.settings, read, write, |_, _| panic!("unexpected commit"));
        assert_eq!(denied.outcome, expected);
        assert_eq!(denied.settings, time.settings);
    }
    assert_eq!(execute(Request::Read, None, Settings::UTC, true, false, |_, _| false).outcome, Outcome::ClockUnavailable);
}
