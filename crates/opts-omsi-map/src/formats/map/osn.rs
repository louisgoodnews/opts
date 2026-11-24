//! The scenario state file of a map (`laststn.osn`): only its `[time]` block is read.
//!
//! The file is the state OMSI saved when the map author last left the map (vehicles, signals,
//! weather link, the clock). Its `[time]` block is the one place a map carries a time of day;
//! `global.cfg` has none. See `docs/src/formats/map/global-cfg.md` (*laststn.osn*).

use crate::formats::text::decode_bytes;

/// The clock of the saved map state, from the five lines after `[time]`: year, day of the year,
/// hour, minute and (fractional) seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OsnTime {
    /// Year (1988 to 2028 in the installed maps).
    pub year: i32,
    /// Day of the year, 1 to 366.
    pub day_of_year: u32,
    /// Hour, 0 to 23.
    pub hour: u32,
    /// Minute, 0 to 59.
    pub minute: u32,
    /// Seconds into the minute.
    pub second: f64,
}

/// How many leading bytes of a `laststn.osn` hold the `[time]` block: in the installed maps it
/// starts within the first 364 bytes of a file of up to 285 KB, so a reader can stop here.
pub const OSN_TIME_PREFIX_BYTES: usize = 4096;

impl OsnTime {
    /// Reads the `[time]` block of a (possibly truncated) `laststn.osn`; `None` when there is no
    /// block or a value is not a number inside its range.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let text = decode_bytes(bytes);
        let mut lines = text.lines().map(str::trim);
        lines.find(|l| l.eq_ignore_ascii_case("[time]"))?;
        let mut number = || -> Option<f64> {
            let value: f64 = lines.next()?.replace(',', ".").parse().ok()?;
            value.is_finite().then_some(value)
        };
        let (year, day, hour, minute) = (number()?, number()?, number()?, number()?);
        let second = number().unwrap_or(0.0);
        let whole = |v: f64, lo: f64, hi: f64| (lo..=hi).contains(&v.trunc()).then_some(v as i64);
        Some(Self {
            year: i32::try_from(whole(year, 1.0, 9999.0)?).ok()?,
            day_of_year: u32::try_from(whole(day, 1.0, 366.0)?).ok()?,
            hour: u32::try_from(whole(hour, 0.0, 23.0)?).ok()?,
            minute: u32::try_from(whole(minute, 0.0, 59.0)?).ok()?,
            second: second.clamp(0.0, 59.999),
        })
    }

    /// The time of day as `(hour, minute)`.
    #[must_use]
    pub fn clock(&self) -> (u32, u32) {
        (self.hour, self.minute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        let mut out = vec![0xFF, 0xFE];
        out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        out
    }

    const BLOCK: &str = "[egopos]\r\n0\r\n\r\n[time]\r\n1988\r\n126\r\n8\r\n45\r\n29.7685798193463\r\n\r\n[vehicles]\r\n";

    #[test]
    fn the_time_block_of_berlin_spandau_is_read() {
        let t = OsnTime::parse(&utf16(BLOCK)).expect("block");
        assert_eq!(
            (t.year, t.day_of_year, t.hour, t.minute),
            (1988, 126, 8, 45)
        );
        assert!((t.second - 29.768).abs() < 1e-3);
        assert_eq!(t.clock(), (8, 45));
    }

    #[test]
    fn utf8_and_a_decimal_comma_work_too() {
        let t = OsnTime::parse("[time]\n2020\n130\n9\n37\n10,5\n".as_bytes()).expect("block");
        assert_eq!(t.clock(), (9, 37));
        assert!((t.second - 10.5).abs() < 1e-9);
    }

    #[test]
    fn a_missing_seconds_line_is_zero() {
        let t = OsnTime::parse("[time]\n2020\n130\n9\n37\n".as_bytes()).expect("block");
        assert_eq!(t.second, 0.0);
    }

    #[test]
    fn a_truncated_file_still_gives_the_block() {
        let full = utf16(&format!("{BLOCK}{}", "x".repeat(10_000)));
        let cut = &full[..OSN_TIME_PREFIX_BYTES];
        assert_eq!(OsnTime::parse(cut).map(|t| t.clock()), Some((8, 45)));
        // A cut inside the block leaves no usable time.
        let early = &utf16(BLOCK)[..2 + 2 * "[egopos]\r\n0\r\n\r\n[time]\r\n19".len()];
        assert_eq!(OsnTime::parse(early), None);
    }

    #[test]
    fn no_block_or_a_value_out_of_range_is_none() {
        assert_eq!(OsnTime::parse(&utf16("[egopos]\r\n0\r\n")), None);
        assert_eq!(OsnTime::parse(&[]), None);
        for bad in [
            "[time]\n2020\n130\n24\n10\n0\n",
            "[time]\n2020\n130\n9\n60\n0\n",
            "[time]\n2020\n0\n9\n10\n0\n",
            "[time]\n2020\n367\n9\n10\n0\n",
            "[time]\n2020\nx\n9\n10\n0\n",
        ] {
            assert_eq!(OsnTime::parse(bad.as_bytes()), None, "{bad:?}");
        }
    }
}
