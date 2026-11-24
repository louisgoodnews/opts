//! Loading a `TTData` folder into [`Timetables`]: the files, their order and the indexes.

use std::path::Path;

use std::collections::HashMap;

use super::{
    parse_busstops, parse_line, parse_stn_links, parse_trip, StnLink, Timetables, TtBusStop,
    TtLine, TtTrip,
};

fn find_ci(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    let want = name.to_ascii_lowercase();
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase() == want)
        })
}

impl Timetables {
    /// Reads `Busstops.cfg`, every `.ttl` and every `.ttp` of a `TTData` folder; a missing folder
    /// gives empty timetables, an unreadable file is skipped.
    #[must_use]
    pub fn load_dir(dir: &Path) -> Self {
        let mut t = Self::default();
        if let Some(p) = find_ci(dir, "Busstops.cfg") {
            if let Ok(b) = std::fs::read(p) {
                t.stops = parse_busstops(&b);
            }
        }
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        files.sort();
        for p in files {
            let ext = p
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let stem = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            match ext.as_str() {
                "ttl" => t.lines.push((stem, parse_line(&bytes))),
                "ttp" => {
                    t.trips
                        .insert(stem.to_ascii_lowercase(), parse_trip(&bytes));
                }
                _ => {}
            }
        }
        if let Some(p) = find_ci(dir, "StnLinks.cfg") {
            if let Ok(b) = std::fs::read(p) {
                t.links = parse_stn_links(&b);
            }
        }
        t.index();
        t
    }

    /// Timetables from parts (tests, tools).
    #[must_use]
    pub fn from_parts(
        stops: Vec<TtBusStop>,
        lines: Vec<(String, TtLine)>,
        trips: Vec<(String, TtTrip)>,
    ) -> Self {
        let mut t = Self {
            stops,
            lines,
            trips: trips
                .into_iter()
                .map(|(n, tr)| (n.to_ascii_lowercase(), tr))
                .collect(),
            stop_by_id: HashMap::new(),
            links: Vec::new(),
            link_by_pair: HashMap::new(),
        };
        t.index();
        t
    }

    /// Adds the station links (tests, tools).
    #[must_use]
    pub fn with_links(mut self, links: Vec<StnLink>) -> Self {
        self.links = links;
        self.index();
        self
    }

    pub(super) fn index(&mut self) {
        self.stop_by_id.clear();
        for (i, s) in self.stops.iter().enumerate() {
            self.stop_by_id.entry(s.id).or_insert(i);
        }
        self.link_by_pair.clear();
        for (i, l) in self.links.iter().enumerate() {
            self.link_by_pair.entry((l.from_id, l.to_id)).or_insert(i);
        }
    }
}
