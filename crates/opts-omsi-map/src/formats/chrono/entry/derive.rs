//! Views derived from a [`ChronoSet`] for one active set: data files, AI lists, summary and switch.

use super::*;

impl ChronoSet {
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

    /// The text of `ailists.cfg` in force: the map's file (the only place a map's list is read
    /// from; an active entry's own `ailists.cfg` replaces it) with the `ailists_#upd.cfg` of every
    /// active entry merged in order ([`merge_ailists`]). `None` when the map has no `ailists.cfg`
    /// and no active entry updates it.
    #[must_use]
    pub fn ailists(&self, active: &ActiveSet) -> Option<String> {
        let mut text = child_ci(&self.map_dir, "ailists.cfg")
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| decode_bytes(&b));
        for e in self.active_entries(active) {
            // A complete list in the entry's own folder replaces the map's.
            if let Some(full) = e
                .files
                .get("ailists.cfg")
                .and_then(|p| std::fs::read(p).ok())
            {
                text = Some(decode_bytes(&full));
            }
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
