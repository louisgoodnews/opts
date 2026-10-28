//! Chrono entries: `Chrono.cfg`, the folder scan, activation by date and the application order.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::data::merge_ailists;
use crate::formats::text::decode_bytes;

/// A calendar date as Chrono writes it (`yyyymmdd`); ordered like the dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ChronoDate(u32);

impl ChronoDate {
    /// The date `year-month-day` (the year is clamped into `0..=9999`).
    #[must_use]
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        let y = u32::try_from(year.clamp(0, 9999)).unwrap_or(0);
        Self(y * 10_000 + month.min(99) * 100 + day.min(99))
    }

    /// Parses `yyyymmdd` (digits only; `None` for anything else, e.g. a blank value).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let t = s.trim();
        if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        t.parse().ok().map(Self)
    }

    /// The date `days` days after 1970-01-01 (negative: before).
    #[must_use]
    pub fn from_unix_days(days: i64) -> Self {
        let (y, m, d) = civil_from_days(days);
        Self::from_ymd(i32::try_from(y).unwrap_or(0), m, d)
    }

    /// The value as written (`19891110`).
    #[must_use]
    pub fn yyyymmdd(self) -> u32 {
        self.0
    }

    /// `(year, month, day)`.
    #[must_use]
    pub fn ymd(self) -> (i32, u32, u32) {
        (
            i32::try_from(self.0 / 10_000).unwrap_or(0),
            self.0 / 100 % 100,
            self.0 % 100,
        )
    }
}

impl std::fmt::Display for ChronoDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// Proleptic Gregorian date of a day number counted from 1970-01-01.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

/// One Chrono entry: a folder below `Chrono/` with a `Chrono.cfg`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChronoEntry {
    /// The entry folder.
    pub dir: PathBuf,
    /// Path of the folder below `Chrono/`, `/`-separated (`0200_Grenzoeffnung`,
    /// `1_FCA Heimspiele/02.11.25 - 14.00 Uhr`).
    pub rel: String,
    /// `[startdate]`: first active day (`None`: from the beginning).
    pub start: Option<ChronoDate>,
    /// `[enddate]`: last active day, inclusive (`None`: open-ended).
    pub end: Option<ChronoDate>,
    /// `[name]` of `Chrono.cfg`.
    pub name: Option<String>,
    /// `[description]` … `[end]` of `Chrono.cfg`.
    pub description: Option<String>,
    /// `[name]` of each `Chrono_<LANG>.dsc`: `(LANG upper-case, name)`.
    pub localized_names: Vec<(String, String)>,
    /// `[deactivate_lines]`: timetable lines switched off while the entry is active.
    pub deactivate_lines: Vec<String>,
    /// `[ticketpack]` (`.otp` paths; the keyword may repeat).
    pub ticketpacks: Vec<String>,
    /// `[moneysystem]` (`Money\…\….cfg`).
    pub moneysystem: Option<String>,
    /// Overlay tiles (`tile_X_Y.map`) by tile key.
    pub tiles: BTreeMap<(i32, i32), PathBuf>,
    /// Tiles marked `<tile>.map.chronoterrrelevant`.
    pub terrain_relevant: BTreeSet<(i32, i32)>,
    /// Other files directly in the folder (data files), by lower-case name.
    pub files: BTreeMap<String, PathBuf>,
    /// Files of the entry's `TTData/` folder, by lower-case name.
    pub ttdata: BTreeMap<String, PathBuf>,
}

impl ChronoEntry {
    /// Reads the fields of a `Chrono.cfg` (or a `Chrono_<LANG>.dsc`, which has `[name]` and
    /// `[description]`). Unknown keywords are ignored.
    pub fn read_cfg(&mut self, bytes: &[u8]) {
        let text = decode_bytes(bytes);
        let lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let mut i = 0;
        while i < lines.len() {
            let Some(kw) = keyword(lines[i]) else {
                i += 1;
                continue;
            };
            i += 1;
            match kw.as_str() {
                "startdate" => {
                    self.start = next_value(&lines, &mut i).and_then(|v| ChronoDate::parse(&v))
                }
                "enddate" => {
                    self.end = next_value(&lines, &mut i).and_then(|v| ChronoDate::parse(&v))
                }
                "name" => self.name = next_value(&lines, &mut i),
                "moneysystem" => self.moneysystem = next_value(&lines, &mut i),
                "ticketpack" => self.ticketpacks.extend(next_value(&lines, &mut i)),
                "deactivate_lines" => {
                    let n = next_value(&lines, &mut i)
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    for _ in 0..n {
                        match next_value(&lines, &mut i) {
                            Some(v) => self.deactivate_lines.push(v),
                            None => break,
                        }
                    }
                }
                "description" => {
                    let mut text = Vec::new();
                    while i < lines.len() {
                        let k = keyword(lines[i]);
                        if k.as_deref()
                            .is_some_and(|k| k == "end" || CFG_KEYWORDS.contains(&k))
                        {
                            break;
                        }
                        text.push(lines[i].trim_end());
                        i += 1;
                    }
                    let joined = text.join("\n").trim().to_string();
                    self.description = (!joined.is_empty()).then_some(joined);
                }
                _ => {}
            }
        }
    }

    /// Whether the entry is active on `date` (`start ≤ date ≤ end`).
    #[must_use]
    pub fn is_active(&self, date: ChronoDate) -> bool {
        self.start.is_none_or(|s| s <= date) && self.end.is_none_or(|e| date <= e)
    }

    /// A name to show: the `.dsc` name in `lang` (`ENG`, `DEU`, …), else `[name]`, else an English
    /// or German `.dsc` name, else the folder name.
    #[must_use]
    pub fn display_name(&self, lang: &str) -> String {
        let localized = |l: &str| {
            self.localized_names
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(l))
                .map(|(_, n)| n.clone())
        };
        localized(lang)
            .or_else(|| self.name.clone())
            .or_else(|| localized("ENG"))
            .or_else(|| localized("DEU"))
            .or_else(|| self.localized_names.first().map(|(_, n)| n.clone()))
            .unwrap_or_else(|| self.rel.rsplit('/').next().unwrap_or_default().to_string())
    }

    /// The key entries are applied in: start date (missing = −∞), then the folder path.
    pub(super) fn order_key(&self) -> (u32, String) {
        (
            self.start.map_or(0, ChronoDate::yyyymmdd),
            self.rel.to_lowercase(),
        )
    }
}

const CFG_KEYWORDS: [&str; 7] = [
    "startdate",
    "enddate",
    "name",
    "moneysystem",
    "ticketpack",
    "deactivate_lines",
    "description",
];

/// The lower-case keyword of a `[keyword]` line.
fn keyword(line: &str) -> Option<String> {
    let t = line.trim_end();
    (t.len() >= 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_ascii_lowercase())
}

/// The next non-blank line before the next keyword, trimmed.
fn next_value(lines: &[&str], i: &mut usize) -> Option<String> {
    while *i < lines.len() {
        if keyword(lines[*i]).is_some() {
            return None;
        }
        let t = lines[*i].trim();
        *i += 1;
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}

/// The tile key of `tile_X_Y.map` (case-insensitive).
#[must_use]
pub(super) fn tile_key_of(name: &str) -> Option<(i32, i32)> {
    let lower = name.to_ascii_lowercase();
    let core = lower.strip_prefix("tile_")?.strip_suffix(".map")?;
    let (x, y) = core.split_once('_')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// The child of `dir` called `name`, compared case-insensitively.
pub(super) fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(name);
    if direct.exists() {
        return Some(direct);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

mod scan;
mod set;

use scan::{files_of, walk};
pub use set::*;

#[cfg(test)]
mod tests;
