//! Frame stepping and counters of the harness.

use super::*;

impl Harness {
    /// The main world (scenario engine and probes).
    pub fn world_mut(&mut self) -> &mut World {
        self.apps.main.world_mut()
    }

    /// Runs one frame and waits for the GPU.
    pub fn update(&mut self) {
        self.apps.update();
        let world = self.apps.main.world();
        if let Some(device) = world.get_resource::<RenderDevice>() {
            let _ = device.wgpu_device().poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            });
        }
    }

    /// Takes the captured events.
    pub fn drain_events(&mut self) -> Vec<super::super::EventRecord> {
        self.sink
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }

    /// A copy of the loader counters.
    #[must_use]
    pub fn stats(&self) -> WorldStats {
        self.apps
            .main
            .world()
            .get_resource::<WorldStats>()
            .cloned()
            .unwrap_or_default()
    }
}
