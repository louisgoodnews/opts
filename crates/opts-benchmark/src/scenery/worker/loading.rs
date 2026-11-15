//! Writing the object's tile and loading the map until it settles.

use super::*;

impl Harness {
    /// Writes the tile (anchor plus `object`, if any) and makes the loader rebuild it from
    /// scratch (parsed objects and meshes are forgotten, so diagnostics repeat per object).
    ///
    /// # Errors
    /// The tile cannot be written.
    pub fn place(&mut self, object: Option<(&str, &[String], Placement)>) -> Result<()> {
        self.map.write_tile(object)?;
        let world = self.apps.main.world_mut();
        let plan = world
            .get_resource_mut::<WorldLoadJob>()
            .map(|mut job| job.plan_scene_reload())
            .ok_or_else(|| anyhow!("no world load job"))?;
        plan.apply(world);
        Ok(())
    }

    /// Loads queued tiles until the tile is stable for two consecutive checks. `Ok(false)` when
    /// `deadline` passes first.
    ///
    /// # Errors
    /// The loading system cannot run.
    pub fn settle(&mut self, deadline: Instant) -> Result<bool> {
        let mut stable = 0;
        while Instant::now() < deadline {
            self.apps
                .main
                .world_mut()
                .run_system_cached(drive_tiles)
                .map_err(|e| anyhow!("drive_tiles: {e}"))?;
            let world = self.apps.main.world();
            let done = world
                .get_resource::<WorldLoadJob>()
                .is_some_and(WorldLoadJob::tiles_done);
            if done {
                stable += 1;
                if stable >= 2 {
                    self.update();
                    return Ok(true);
                }
            } else {
                stable = 0;
            }
            self.update();
        }
        Ok(false)
    }
}
