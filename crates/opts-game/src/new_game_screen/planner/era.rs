//! What a map says about its time: its years, its year offset and its saved start clock.

use bevy::prelude::*;

use opts_bevy::CalendarDate;

use crate::shell::calendar;
use super::TimeSource;

/// What a map's `global.cfg` says about its time: `[years]` and `[realyearoffset]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapEra {
    /// `[years]`: the first and last year the map is set in.
    pub years: Option<(i32, i32)>,
    /// `[realyearoffset]`: years added to today's date (0 when absent).
    pub real_year_offset: i32,
    /// The time of day (hour, minute) of the map's `laststn.osn` `[time]` block, if it has one.
    pub start_clock: Option<(u32, u32)>,
}

impl MapEra {
    /// The era of a parsed `global.cfg`.
    pub fn of(manifest: &opts_core::formats::map::MapManifest) -> Self {
        Self {
            years: manifest.years.map(|(a, b)| (a.min(b), a.max(b))),
            real_year_offset: manifest.real_year_offset.unwrap_or(0),
            start_clock: None,
        }
    }

    /// Reads `maps/<folder>/global.cfg` and the start of `laststn.osn` below the installation;
    /// `None` when neither tells anything.
    pub fn read(root: &std::path::Path, folder: &str) -> Option<Self> {
        let dir = root.join("maps").join(folder);
        let era = std::fs::read(dir.join("global.cfg"))
            .ok()
            .and_then(|bytes| opts_core::formats::map::MapManifest::parse(&bytes).ok())
            .map(|m| Self::of(&m));
        let start_clock = read_start_clock(&dir);
        if era.is_none() && start_clock.is_none() {
            return None;
        }
        Some(Self {
            start_clock,
            ..era.unwrap_or_default()
        })
    }
}

/// The time of day of `<map dir>/laststn.osn` (`[time]`): only the first few KB are read.
fn read_start_clock(map_dir: &std::path::Path) -> Option<(u32, u32)> {
    use opts_core::formats::map::{OsnTime, OSN_TIME_PREFIX_BYTES};
    use std::io::Read;
    let mut prefix = Vec::new();
    std::fs::File::open(map_dir.join("laststn.osn"))
        .ok()?
        .take(OSN_TIME_PREFIX_BYTES as u64)
        .read_to_end(&mut prefix)
        .ok()?;
    OsnTime::parse(&prefix).map(|t| t.clock())
}

/// The time of day a plan starts at when the user has not chosen one: the map's saved clock,
/// else `now`; with the source for the label.
pub fn default_start_clock(era: &MapEra, now: (u32, u32)) -> ((u32, u32), TimeSource) {
    match era.start_clock {
        Some(clock) => (clock, TimeSource::Map),
        None => (now, TimeSource::Now),
    }
}

/// The date a map starts on when the user has not chosen one: today shifted by
/// `[realyearoffset]` years (29 February becomes 28 February in a common year), clamped into
/// `[years]` as `a-01-01 ..= b-12-31`. A map without `[years]` keeps the shifted date.
pub fn default_map_date(today: CalendarDate, era: &MapEra) -> CalendarDate {
    let shifted = CalendarDate::clamped(today.year + era.real_year_offset, today.month, today.day);
    let date = match era.years {
        Some((a, b)) => shifted.clamp(CalendarDate::new(a, 1, 1), CalendarDate::new(b, 12, 31)),
        None => shifted,
    };
    calendar::clamp_date(date)
}

/// The map's years suggestion: `Some((from, to))` when the chosen year lies outside them.
pub fn era_hint(years: &str, year: i32) -> Option<(i32, i32)> {
    let nums: Vec<i32> = years
        .split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .filter_map(|p| p.parse().ok())
        .collect();
    let from = *nums.iter().min()?;
    let to = *nums.iter().max()?;
    (year < from || year > to).then_some((from, to))
}
