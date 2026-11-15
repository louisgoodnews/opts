//! Read-only probes of the mirror scene: the live state, the cameras and their activity.

use super::*;

impl Scene {
    pub(super) fn quality(&mut self, count: u8, every: u8) {
        let mut q = self.app.world_mut().resource_mut::<LightQuality>();
        q.mirror_count = count;
        q.mirror_every = every;
    }

    pub(super) fn cameras(&mut self) -> Vec<Entity> {
        let mut q = self
            .app
            .world_mut()
            .query_filtered::<Entity, With<MirrorCamera>>();
        q.iter(self.app.world()).collect()
    }

    pub(super) fn active(&self, camera: Entity) -> bool {
        self.app.world().get::<Camera>(camera).unwrap().is_active
    }

    pub(super) fn camera_of(&self, mirror: Entity) -> Entity {
        self.app
            .world()
            .get::<MirrorSurface>(mirror)
            .unwrap()
            .camera
    }

    pub(super) fn shows_live(&self, mirror: Entity) -> bool {
        let camera = self.camera_of(mirror);
        let info = self.app.world().get::<MirrorCamera>(camera).unwrap();
        self.app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(mirror)
            .unwrap()
            .0
            == info.dynamic
    }
}
