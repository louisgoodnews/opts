//! The sync system: keeps the one shared cube, and the camera light that reads it, in step with
//! the quality and the game's sky input.

use bevy::light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy::prelude::*;

use super::input::{RegenSnapshot, SkyCubeInput};
use super::policy::{exposure, should_regenerate};
use super::texels::{cube_data, cube_image};
use super::tuning::{BASE_INTENSITY, CUBE_SIZE};
use crate::light_quality::{LightQuality, LightQualitySet};
use crate::MainCamera;

/// The one shared cube and when it was built.
#[derive(Resource, Default)]
pub struct SkyCubeState {
    /// The cube image while reflections are on.
    pub handle: Option<Handle<Image>>,
    last: Option<(RegenSnapshot, f64)>,
    /// How often the cube was built since start-up.
    pub builds: u32,
    /// The intensity last written to the camera.
    pub intensity: f32,
}

/// Marks a camera that carries the sky cube light.
#[derive(Component)]
pub struct SkyCubeApplied;

type SkyCubeCameraItem<'a> = (
    Entity,
    Has<SkyCubeApplied>,
    Option<&'a mut GeneratedEnvironmentMapLight>,
    Option<&'a mut EnvironmentMapLight>,
);

/// System parameters for syncing the sky cube.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SkyCubeParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub quality: Res<'w, LightQuality>,
    pub input: Option<Res<'w, SkyCubeInput>>,
    pub cabin: Option<Res<'w, crate::light_rooms::CabinView>>,
    pub state: ResMut<'w, SkyCubeState>,
    pub images: ResMut<'w, Assets<Image>>,
    pub cameras: Query<'w, 's, SkyCubeCameraItem<'static>, With<MainCamera>>,
}

fn update_cube_texture(
    input: &SkyCubeInput,
    state: &mut SkyCubeState,
    images: &mut Assets<Image>,
    now_s: f64,
) -> Handle<Image> {
    let now = input.snapshot();
    let last = state.last.as_ref().map(|(s, t)| (s, *t));
    match state.handle.clone() {
        Some(h) if images.contains(&h) => {
            if should_regenerate(last, &now, now_s) {
                if let Some(mut img) = images.get_mut(&h) {
                    img.data = Some(cube_data(input, CUBE_SIZE));
                }
                state.last = Some((now, now_s));
                state.builds += 1;
            }
            h
        }
        _ => {
            let h = images.add(cube_image(input, CUBE_SIZE));
            state.handle = Some(h.clone());
            state.last = Some((now, now_s));
            state.builds += 1;
            h
        }
    }
}

/// Keeps the cube and the camera light in step with the quality and the input.
pub fn sync_sky_cube(mut p: SkyCubeParams, mut commands: Commands) {
    let input = p.input.filter(|_| p.quality.glass_reflections);
    let Some(input) = input else {
        for (entity, applied, ..) in &p.cameras {
            if applied {
                commands.entity(entity).remove::<(
                    SkyCubeApplied,
                    GeneratedEnvironmentMapLight,
                    EnvironmentMapLight,
                )>();
            }
        }
        if p.state.handle.take().is_some() {
            p.state.last = None;
        }
        return;
    };
    let now_s = p.time.elapsed_secs_f64();
    let handle = update_cube_texture(&input, &mut p.state, &mut p.images, now_s);
    let interior = p.cabin.is_some_and(|c| c.interior_view);
    let intensity = BASE_INTENSITY
        * exposure(input.sun_elevation_deg, input.ambient)
        * if interior {
            crate::light_rooms::INTERIOR_AMBIENT_SCALE
        } else {
            1.0
        };
    p.state.intensity = intensity;
    for (entity, applied, generated, probe) in &mut p.cameras {
        if !applied {
            commands.entity(entity).insert((
                SkyCubeApplied,
                GeneratedEnvironmentMapLight {
                    environment_map: handle.clone(),
                    intensity,
                    ..default()
                },
            ));
            continue;
        }
        if let Some(mut g) = generated {
            if (g.intensity - intensity).abs() > 0.5 {
                g.intensity = intensity;
            }
        }
        if let Some(mut probe) = probe {
            if (probe.intensity - intensity).abs() > 0.5 {
                probe.intensity = intensity;
            }
        }
    }
}

/// Registers [`sync_sky_cube`].
#[derive(Default)]
pub struct SkyCubePlugin;

impl Plugin for SkyCubePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .init_resource::<SkyCubeState>()
            .add_systems(
                Update,
                sync_sky_cube
                    .after(LightQualitySet)
                    .run_if(resource_exists::<Assets<Image>>),
            );
    }
}
