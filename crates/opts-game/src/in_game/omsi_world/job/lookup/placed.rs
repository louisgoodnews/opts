//! The placed object with an ID code in the loaded tiles: a placed object or one attached to a
//! spline, in the tile frame.

use super::*;

impl WorldLoadJob {
    /// The object with this ID code in the first of `keys` that has it: placed in its tile, or
    /// attached to a spline (`[splineAttachement]`, e.g. `entrypoint_bus.sco` along a bus lane
    /// in Wuppertal 639, or a bus stop along a road), whose place follows from the carrier
    /// spline. Position (tile frame), heading and `.sco` file.
    pub(super) fn find_placed_object(
        &mut self,
        id_code: u64,
        keys: &[(i32, i32)],
    ) -> Option<((i32, i32), Vec3, f32, String)> {
        for &key in keys {
            let Some(tile) = self.read_tile(key) else {
                continue;
            };
            if let Some(obj) = tile.objects.iter().find(|o| o.id_code == id_code) {
                return Some((key, obj.position, obj.heading_deg, obj.sco_file.clone()));
            }
            if let Some(att) = tile
                .spline_attachments
                .iter()
                .find(|a| a.id_code == id_code)
            {
                let index = self.spline_index();
                let placed = crate::in_game::attachments::expand_attachments(
                    std::slice::from_ref(att),
                    &tile.splines,
                    &index,
                    key,
                    &self.grid,
                    |_, _| 0.0,
                );
                if let Some((obj, _)) = placed.first() {
                    return Some((key, obj.position, obj.heading_deg, obj.sco_file.clone()));
                }
            }
        }
        None
    }
}
