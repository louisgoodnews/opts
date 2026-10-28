//! Unit tests for `super` (`update.rs`).

use super::*;

#[test]
fn times_and_delays_read_as_a_timetable_does() {
    assert_eq!(hhmm(5.0 * 3600.0 + 27.0 * 60.0), "5:27");
    assert_eq!(hhmm(86_400.0 + 61.0), "0:01");
    assert_eq!(delay_text(74.0), "+1:14");
    assert_eq!(delay_text(-30.0), "-0:30");
    assert_eq!(delay_text(0.0), "+0:00");
    assert_eq!(delay_text(23_619.0), "+6:33:39");
}
