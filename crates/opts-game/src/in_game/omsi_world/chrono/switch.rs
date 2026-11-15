//! The live switch to another map date: the active sets are compared and the changed tiles
//! are rebuilt.

use super::*;

impl WorldLoadJob {
    /// The live switch to `date`: plans and starts the rebuild (see the module docs). A date with
    /// the same active entries only moves the map date.
    pub(in crate::in_game::omsi_world) fn switch_chrono(
        &mut self,
        date: ChronoDate,
        player: Option<(i32, i32)>,
    ) -> SwitchOutcome {
        let old = self.chrono.active.clone();
        let new = self.chrono.set.active(date);
        let switch = self.chrono.set.switch(&old, &new);
        self.set_chrono_date(Some(date));
        if switch.is_noop() {
            tracing::info!(date = %date, "map date changed; same Chrono entries, nothing to rebuild");
            return SwitchOutcome::default();
        }
        let mut tiles = switch.rebuild_tiles();
        if switch.touches_prefix("parklist_p") {
            // Parked cars come from the list: every resident tile may change.
            self.parklists.clear();
            tiles.extend(self.resident.keys().copied());
        }
        for key in &tiles {
            self.tile_splines.remove(key);
            self.tile_aligned.remove(key);
            self.tile_objects.remove(key);
            self.tile_surfaces.remove(key);
        }
        self.spline_index = None;
        let order = rebuild_order(&tiles, |k| self.resident.contains_key(&k), player);
        let unloads: Vec<TileUnload> = order
            .iter()
            .filter_map(|k| self.unload_tile(*k, Some(*k) == player))
            .collect();
        for key in order.iter().rev() {
            self.queue.retain(|q| q != key);
            self.queue.push_front(*key);
        }
        self.chrono.rebuilding.clone_from(&order);
        tracing::info!(
            date = %date,
            toggled = switch.toggled.len(),
            changed_tiles = switch.tiles.len(),
            carve_ring = switch.carve_ring.len(),
            rebuild = order.len(),
            "Chrono switch"
        );
        SwitchOutcome {
            rebuild: order,
            unloads,
            traffic: switch.touches("ailists_#upd.cfg")
                || switch.touches("ailists.cfg")
                || switch.touches("unsched_trafficdens.txt"),
        }
    }

    /// Tiles of the running switch still to rebuild (a tile is done once it is resident again or
    /// no longer queued, e.g. a tile that exists no more).
    pub(in crate::in_game::omsi_world) fn chrono_pending(&mut self) -> usize {
        let (resident, queue) = (&self.resident, &self.queue);
        let building = self.building.as_ref().map(|b| b.key);
        self.chrono.rebuilding.retain(|k| {
            let in_progress = building == Some(*k);
            in_progress || (!resident.contains_key(k) && queue.contains(k))
        });
        self.chrono.rebuilding.len()
    }
}
