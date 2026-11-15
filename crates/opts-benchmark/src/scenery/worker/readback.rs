//! Rendering the capture views and reading the pixels back.

use super::*;

impl Harness {
    /// Renders one frame through the capture cameras without reading it back. Entities that
    /// were hidden until now (light sprites) have their meshes and materials prepared for the
    /// GPU only once they are drawn, which takes one frame; a capture right after they appear
    /// would miss them.
    pub fn warm_up(&mut self, views: &[View]) {
        let world = self.apps.main.world_mut();
        self.rig.aim(world, views);
        self.rig.set_active(world, true);
        self.update();
        let world = self.apps.main.world_mut();
        self.rig.set_active(world, false);
    }

    /// Renders both views over black and white and reads them back.
    ///
    /// # Errors
    /// The readbacks do not arrive within a few frames.
    pub fn capture(&mut self, views: &[View; 2]) -> Result<[ViewPixels; 2]> {
        let [a, b]: [ViewPixels; 2] = self
            .capture_views(views)?
            .try_into()
            .map_err(|_| anyhow!("unexpected number of views"))?;
        Ok([a, b])
    }

    /// Renders up to [`super::super::capture::MAX_VIEWS`] views over black and white and reads them
    /// back.
    ///
    /// # Errors
    /// The readbacks do not arrive within a few frames.
    pub fn capture_views(&mut self, views: &[View]) -> Result<Vec<ViewPixels>> {
        let world = self.apps.main.world_mut();
        self.rig.aim(world, views);
        let readers = self.rig.request(world);
        self.update();
        let world = self.apps.main.world_mut();
        self.rig.set_active(world, false);
        self.rig.stop_reading(world, &readers);
        let mut out = None;
        for _ in 0..5 {
            self.update();
            out = self.rig.take();
            if out.is_some() {
                break;
            }
        }
        self.rig.despawn(self.apps.main.world_mut(), readers);
        out.ok_or_else(|| anyhow!("readback did not arrive"))
    }
}
