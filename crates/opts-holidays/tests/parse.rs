//! Date parsing of holiday records.

use opts_holidays::parse_date;

#[test]
fn eight_digit_dates_with_a_month_and_a_day_in_range_parse() {
    assert_eq!(parse_date("20261225"), Some(20261225));
    assert_eq!(parse_date("20260101"), Some(20260101));
}

#[test]
fn other_shapes_and_ranges_do_not_parse() {
    assert_eq!(parse_date("2026125"), None, "seven digits");
    assert_eq!(parse_date("2026-12-25"), None, "separators");
    assert_eq!(parse_date("20261300"), None, "month 13");
    assert_eq!(parse_date("20261200"), None, "day 0");
    assert_eq!(parse_date("20261232"), None, "day 32");
    assert_eq!(parse_date(""), None);
}
