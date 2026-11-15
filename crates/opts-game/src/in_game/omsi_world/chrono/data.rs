//! Data files of the map (`parklist`, `registrations`, `ailists`, ...) read through the Chrono plan,
//! and the Chrono state exposed to consumers.

use super::*;

impl WorldLoadJob {
    /// The file a map-level data file is read from: the replacement of the last active Chrono
    /// entry that has one, else the map's own (`parklist_p*.txt`, `registrations.txt`,
    /// `unsched_trafficdens.txt`, `unsched_vehgroups.txt`, …). Names compare case-insensitively.
    #[must_use]
    pub fn data_file(&self, name: &str) -> Option<PathBuf> {
        self.chrono
            .set
            .effective_file(&self.chrono.active, name)
            .or_else(|| self.install.find_in(&self.config.map_dir, name))
    }

    /// The contents of a map-level data file in force: `ailists.cfg` with the group updates of the
    /// active entries merged in, any other file from [`Self::data_file`].
    #[must_use]
    pub fn data_bytes(&self, name: &str) -> Option<Vec<u8>> {
        if name.eq_ignore_ascii_case("ailists.cfg") {
            return self
                .chrono
                .set
                .ailists(&self.chrono.active)
                .map(String::into_bytes)
                .or_else(|| {
                    self.install
                        .find_in(&self.config.map_dir, name)
                        .and_then(|p| std::fs::read(p).ok())
                });
        }
        self.data_file(name).and_then(|p| std::fs::read(p).ok())
    }

    /// The timetable files in force (map `TTData/` overlaid by the active entries), by
    /// lower-case name. The game does not use timetables yet.
    #[must_use]
    pub fn ttdata_files(&self) -> BTreeMap<String, PathBuf> {
        self.chrono.set.ttdata(&self.chrono.active)
    }

    /// The Chrono state for [`ChronoState`].
    #[must_use]
    pub fn chrono_state(&self) -> ChronoState {
        let c = &self.chrono;
        let summary = c.set.summary(&c.active);
        let active: Vec<_> = c
            .active
            .indices()
            .iter()
            .filter_map(|&i| c.set.entries().get(i))
            .collect();
        ChronoState {
            date: c.date,
            active: active.iter().map(|e| e.rel.clone()).collect(),
            names: active.iter().map(|e| e.display_name("ENG")).collect(),
            deactivate_lines: summary.deactivate_lines,
            ticketpacks: summary.ticketpacks,
            moneysystem: summary.moneysystem,
        }
    }
}
