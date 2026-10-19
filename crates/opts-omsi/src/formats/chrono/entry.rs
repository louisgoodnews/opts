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
    fn order_key(&self) -> (u32, String) {
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
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
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

/// Indices of the active entries of a [`ChronoSet`], in application order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActiveSet(Vec<usize>);

impl ActiveSet {
    /// Entry indices in application order.
    #[must_use]
    pub fn indices(&self) -> &[usize] {
        &self.0
    }

    /// Whether entry `i` is active.
    #[must_use]
    pub fn contains(&self, i: usize) -> bool {
        self.0.contains(&i)
    }

    /// Whether no entry is active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Which overlays apply to which tile for one [`ActiveSet`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoPlan {
    /// Overlay tile files per tile, in application order.
    pub tiles: BTreeMap<(i32, i32), Vec<PathBuf>>,
    /// Tiles with a `.chronoterrrelevant` marker in an active entry.
    pub terrain_relevant: BTreeSet<(i32, i32)>,
}

impl ChronoPlan {
    /// The overlays of a tile, in application order (empty for an untouched tile).
    #[must_use]
    pub fn overlays(&self, key: (i32, i32)) -> &[PathBuf] {
        self.tiles.get(&key).map_or(&[], Vec::as_slice)
    }
}

/// What the active entries set besides tiles and data files (exposed; no consumer yet).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoSummary {
    /// `[deactivate_lines]` of all active entries (in order, without duplicates).
    pub deactivate_lines: Vec<String>,
    /// `[ticketpack]` of the last active entry that names one.
    pub ticketpacks: Vec<String>,
    /// `[moneysystem]` of the last active entry that names one.
    pub moneysystem: Option<String>,
}

/// The difference between two active sets: what a live switch has to rebuild.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChronoSwitch {
    /// Entries active in exactly one of the two sets.
    pub toggled: Vec<usize>,
    /// Tiles whose overlay list differs.
    pub tiles: BTreeSet<(i32, i32)>,
    /// The 3 × 3 neighbourhoods of tiles marked `.chronoterrrelevant` in a toggled entry: their
    /// terrain carve has to be recomputed.
    pub carve_ring: BTreeSet<(i32, i32)>,
    /// Lower-case names of the data files in toggled entries (`unsched_trafficdens.txt`,
    /// `ailists_#upd.cfg`, `parklist_p.txt`, …).
    pub data_files: BTreeSet<String>,
    /// Whether a toggled entry has a `TTData/` folder.
    pub ttdata: bool,
}

impl ChronoSwitch {
    /// Whether nothing changes.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.toggled.is_empty()
    }

    /// Whether a data file called `name` (case-insensitive) changes.
    #[must_use]
    pub fn touches(&self, name: &str) -> bool {
        self.data_files.contains(&name.to_lowercase())
    }

    /// Whether a data file whose name starts with `prefix` (case-insensitive) changes.
    #[must_use]
    pub fn touches_prefix(&self, prefix: &str) -> bool {
        let p = prefix.to_lowercase();
        self.data_files.iter().any(|f| f.starts_with(&p))
    }

    /// Tiles to rebuild: the changed tiles and the carve ring.
    #[must_use]
    pub fn rebuild_tiles(&self) -> BTreeSet<(i32, i32)> {
        self.tiles.union(&self.carve_ring).copied().collect()
    }
}

/// The Chrono entries of a map, in application order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChronoSet {
    map_dir: PathBuf,
    entries: Vec<ChronoEntry>,
}

impl ChronoSet {
    /// A set without entries (a map without `Chrono/`).
    #[must_use]
    pub fn empty(map_dir: &Path) -> Self {
        Self {
            map_dir: map_dir.to_path_buf(),
            entries: Vec::new(),
        }
    }

    /// Scans `<map_dir>/Chrono/` recursively (folder names compared case-insensitively). Unreadable
    /// folders and files are skipped.
    #[must_use]
    pub fn scan(map_dir: &Path) -> Self {
        let mut set = Self::empty(map_dir);
        if let Some(dir) = child_ci(map_dir, "Chrono").filter(|d| d.is_dir()) {
            walk(&dir, "", &mut set.entries);
        }
        set.entries.sort_by_cached_key(ChronoEntry::order_key);
        set
    }

    /// Builds a set from entries (sorted into application order).
    #[must_use]
    pub fn from_entries(map_dir: &Path, mut entries: Vec<ChronoEntry>) -> Self {
        entries.sort_by_cached_key(ChronoEntry::order_key);
        Self {
            map_dir: map_dir.to_path_buf(),
            entries,
        }
    }

    /// The map folder.
    #[must_use]
    pub fn map_dir(&self) -> &Path {
        &self.map_dir
    }

    /// The entries in application order.
    #[must_use]
    pub fn entries(&self) -> &[ChronoEntry] {
        &self.entries
    }

    /// The entries active on `date`.
    #[must_use]
    pub fn active(&self, date: ChronoDate) -> ActiveSet {
        ActiveSet(
            (0..self.entries.len())
                .filter(|&i| self.entries[i].is_active(date))
                .collect(),
        )
    }

    /// Every entry with its state on `date`, in application order.
    #[must_use]
    pub fn states(&self, date: ChronoDate) -> Vec<(&ChronoEntry, bool)> {
        self.entries
            .iter()
            .map(|e| (e, e.is_active(date)))
            .collect()
    }

    fn active_entries<'a>(
        &'a self,
        active: &'a ActiveSet,
    ) -> impl DoubleEndedIterator<Item = &'a ChronoEntry> {
        active.0.iter().filter_map(|&i| self.entries.get(i))
    }

    /// The overlays per tile and the carve markers of an active set.
    #[must_use]
    pub fn plan(&self, active: &ActiveSet) -> ChronoPlan {
        let mut plan = ChronoPlan::default();
        for e in self.active_entries(active) {
            for (key, path) in &e.tiles {
                plan.tiles.entry(*key).or_default().push(path.clone());
            }
            plan.terrain_relevant
                .extend(e.terrain_relevant.iter().copied());
        }
        plan
    }

    /// The file a map-level data file is read from: that of the last active entry that has it
    /// (`parklist_p*.txt`, `registrations.txt`, `unsched_trafficdens.txt` replace the whole
    /// file), else the map's own (`None` when neither exists). Names compare case-insensitively.
    #[must_use]
    pub fn effective_file(&self, active: &ActiveSet, name: &str) -> Option<PathBuf> {
        let lower = name.to_lowercase();
        self.active_entries(active)
            .rev()
            .find_map(|e| e.files.get(&lower).cloned())
            .or_else(|| child_ci(&self.map_dir, name).filter(|p| p.is_file()))
    }

    /// The timetable files in force: the map's `TTData/` overlaid file by file by the active
    /// entries' `TTData/` (by lower-case name).
    #[must_use]
    pub fn ttdata(&self, active: &ActiveSet) -> BTreeMap<String, PathBuf> {
        let mut out = BTreeMap::new();
        if let Some(dir) = child_ci(&self.map_dir, "TTData").filter(|d| d.is_dir()) {
            out.extend(files_of(&dir));
        }
        for e in self.active_entries(active) {
            out.extend(e.ttdata.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        out
    }

    /// The text of `ailists.cfg` in force: the map's file with the `ailists_#upd.cfg` of every
    /// active entry merged in order ([`merge_ailists`]). `None` when the map has no `ailists.cfg`
    /// and no active entry updates it.
    #[must_use]
    pub fn ailists(&self, active: &ActiveSet) -> Option<String> {
        let mut text = child_ci(&self.map_dir, "ailists.cfg")
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| decode_bytes(&b));
        for e in self.active_entries(active) {
            let Some(update) = e
                .files
                .get("ailists_#upd.cfg")
                .and_then(|p| std::fs::read(p).ok())
            else {
                continue;
            };
            let base = text.take().unwrap_or_default();
            text = Some(merge_ailists(&base, &decode_bytes(&update)));
        }
        text
    }

    /// Deactivated lines, ticket pack and money system of an active set.
    #[must_use]
    pub fn summary(&self, active: &ActiveSet) -> ChronoSummary {
        let mut out = ChronoSummary::default();
        for e in self.active_entries(active) {
            for l in &e.deactivate_lines {
                if !out
                    .deactivate_lines
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(l))
                {
                    out.deactivate_lines.push(l.clone());
                }
            }
            if !e.ticketpacks.is_empty() {
                out.ticketpacks.clone_from(&e.ticketpacks);
            }
            if e.moneysystem.is_some() {
                out.moneysystem.clone_from(&e.moneysystem);
            }
        }
        out
    }

    /// What changes between two active sets.
    #[must_use]
    pub fn switch(&self, old: &ActiveSet, new: &ActiveSet) -> ChronoSwitch {
        let mut out = ChronoSwitch::default();
        if old == new {
            return out;
        }
        out.toggled = (0..self.entries.len())
            .filter(|&i| old.contains(i) != new.contains(i))
            .collect();
        let (a, b) = (self.plan(old), self.plan(new));
        for key in a.tiles.keys().chain(b.tiles.keys()) {
            if a.overlays(*key) != b.overlays(*key) {
                out.tiles.insert(*key);
            }
        }
        for &i in &out.toggled {
            let e = &self.entries[i];
            for &(x, y) in &e.terrain_relevant {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        out.carve_ring.insert((x + dx, y + dy));
                    }
                }
            }
            out.data_files.extend(e.files.keys().cloned());
            out.ttdata |= !e.ttdata.is_empty();
        }
        out
    }
}

mod scan;
use scan::{files_of, walk};

#[cfg(test)]
mod tests;
