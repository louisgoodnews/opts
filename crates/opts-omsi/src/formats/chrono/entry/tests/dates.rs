//! Date parsing, ordering and conversion of `entry`.

use super::super::*;

#[test]
fn dates_parse_order_and_convert() {
    assert_eq!(
        ChronoDate::parse("19891110"),
        Some(ChronoDate::from_ymd(1989, 11, 10))
    );
    assert_eq!(ChronoDate::parse(""), None);
    assert_eq!(ChronoDate::parse("1989-11-10"), None);
    assert!(ChronoDate::from_ymd(1989, 11, 10) < ChronoDate::from_ymd(1990, 1, 1));
    // 2000-01-01 is day 10957 after 1970-01-01.
    assert_eq!(
        ChronoDate::from_unix_days(10_957),
        ChronoDate::from_ymd(2000, 1, 1)
    );
    assert_eq!(
        ChronoDate::from_unix_days(10_957 - 4629),
        ChronoDate::from_ymd(1987, 4, 30)
    );
    assert_eq!(ChronoDate::from_unix_days(0).to_string(), "1970-01-01");
    assert_eq!(ChronoDate::from_unix_days(-1).to_string(), "1969-12-31");
    assert_eq!(ChronoDate::from_unix_days(11_016).to_string(), "2000-02-29");
}
