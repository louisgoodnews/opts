//! Off-screen capture: four cameras (two views × black and white background) render into image
//! targets that are read back with `Readback` (Q-SB-01: works with no window at all).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use bevy::camera::{
    OrthographicProjection, PerspectiveProjection, Projection as BevyProjection, ScalingMode,
};
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};

use super::framing::{Projection, View, IMAGE_SIZE};

mod spawn;

/// Most views one capture takes (the static check uses two, the script check three).
pub const MAX_VIEWS: usize = 3;
/// Cameras: every view over black and over white.
const CAMERAS: usize = 2 * MAX_VIEWS;

/// Readback results by camera index.
type Sink = Arc<Mutex<Vec<(usize, Vec<u8>)>>>;

/// RGBA8 sRGB pixels of one view over both backgrounds.
#[derive(Debug, Clone)]
pub struct ViewPixels {
    /// Over black.
    pub black: Vec<u8>,
    /// Over white.
    pub white: Vec<u8>,
}

/// The four cameras, their targets and the light.
#[derive(Debug)]
pub struct CaptureRig {
    cameras: [Entity; CAMERAS],
    images: [Handle<Image>; CAMERAS],
    /// Views the cameras are aimed at in the current capture.
    views: AtomicUsize,
    light: Entity,
    sink: Sink,
}

impl CaptureRig {
    /// Points the cameras at `views` (at most [`MAX_VIEWS`]) and the light from V1's upper left.
    pub fn aim(&self, world: &mut World, views: &[View]) {
        let n = views.len().min(MAX_VIEWS);
        self.views.store(n, Ordering::Relaxed);
        for (i, cam) in self.cameras.iter().enumerate().take(2 * n) {
            let view = &views[i / 2];
            let projection = match view.projection {
                Projection::Perspective { fov_y, near, far } => {
                    BevyProjection::Perspective(PerspectiveProjection {
                        fov: fov_y,
                        aspect_ratio: 1.0,
                        near,
                        far,
                        ..default()
                    })
                }
                Projection::Orthographic { extent, near, far } => {
                    BevyProjection::Orthographic(OrthographicProjection {
                        near,
                        far,
                        scaling_mode: ScalingMode::Fixed {
                            width: extent,
                            height: extent,
                        },
                        ..OrthographicProjection::default_3d()
                    })
                }
            };
            let transform = view.transform();
            world.entity_mut(*cam).insert((projection, transform));
        }
        let v1 = &views[0];
        let forward = (v1.target - v1.eye).normalize_or_zero();
        let left = Vec3::Y.cross(forward).normalize_or_zero();
        let from = v1.target - forward + left * 0.6 + Vec3::Y * 1.2;
        let light = Transform::from_translation(from).looking_at(v1.target, Vec3::Y);
        world.entity_mut(self.light).insert(light);
    }

    /// Activates the cameras and starts one readback per target; returns the reader entities.
    ///
    /// `ReadbackOnce` is not used: its cleanup runs in `First` of the next update, which would
    /// remove a readback requested between updates before it is ever extracted.
    pub fn request(&self, world: &mut World) -> Vec<Entity> {
        if let Ok(mut v) = self.sink.lock() {
            v.clear();
        }
        self.set_active(world, true);
        self.images
            .iter()
            .enumerate()
            .take(2 * self.views.load(Ordering::Relaxed))
            .map(|(i, image)| {
                let sink = self.sink.clone();
                world
                    .spawn(Readback::texture(image.clone()))
                    .observe(move |ev: On<ReadbackComplete>| {
                        if let Ok(mut v) = sink.lock() {
                            v.push((i, ev.data.clone()));
                        }
                    })
                    .id()
            })
            .collect()
    }

    /// Stops further readbacks (the ones in flight still arrive).
    pub fn stop_reading(&self, world: &mut World, readers: &[Entity]) {
        for e in readers {
            if let Ok(mut entity) = world.get_entity_mut(*e) {
                entity.remove::<Readback>();
            }
        }
    }

    /// Removes the reader entities.
    pub fn despawn(&self, world: &mut World, readers: Vec<Entity>) {
        for e in readers {
            if let Ok(entity) = world.get_entity_mut(e) {
                entity.despawn();
            }
        }
    }

    /// Deactivates the cameras (no rendering while objects load).
    pub fn set_active(&self, world: &mut World, active: bool) {
        let n = 2 * self.views.load(Ordering::Relaxed);
        for (i, cam) in self.cameras.iter().enumerate() {
            if let Some(mut c) = world.get_mut::<Camera>(*cam) {
                c.is_active = active && i < n;
            }
        }
    }

    /// The views' pixels once all their readbacks arrived.
    #[must_use]
    pub fn take(&self) -> Option<Vec<ViewPixels>> {
        let got = self.sink.lock().ok()?;
        let n = 2 * self.views.load(Ordering::Relaxed);
        let find = |i: usize| got.iter().find(|(j, _)| *j == i).map(|(_, d)| d.clone());
        let expected = (IMAGE_SIZE * IMAGE_SIZE * 4) as usize;
        let all: Vec<Vec<u8>> = (0..n).filter_map(find).collect();
        if all.len() != n || all.iter().any(|d| d.len() < expected) {
            return None;
        }
        let mut it = all.into_iter();
        Some(
            (0..n / 2)
                .map(|_| ViewPixels {
                    black: it.next().unwrap_or_default(),
                    white: it.next().unwrap_or_default(),
                })
                .collect(),
        )
    }
}
