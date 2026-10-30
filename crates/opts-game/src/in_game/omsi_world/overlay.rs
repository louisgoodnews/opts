//! Toggleable diagnostic overlay.

use bevy::prelude::*;
use opts_core::formats::map::TileGrid;

use crate::in_game::types::InGameEntity;

/// Asset stores the loading systems write to (grouped to stay within Bevy's parameter limit).
use super::{PositionReadout, WorldPos, WorldStats};

/// Marker for the toggleable diagnostic overlay text.
#[derive(Component)]
pub struct DebugOverlayText;

/// Spawns the developer panel (hidden unless *Settings → Advanced → Developer tools* is on). It
/// uses the instrument tokens like the rest of the HUD.
pub fn spawn_debug_overlay(commands: &mut Commands) {
    commands.spawn((
        (
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(96.0),
                left: Val::Px(20.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Visibility::Hidden,
        ),
        crate::in_game::instruments::InstrumentPanel,
        crate::in_game::instruments::InstrumentInk::Text,
        DebugOverlayText,
        InGameEntity,
    ));
}

/// F10 switches the developer panel by changing its setting (so the page and the key agree and
/// the choice is remembered).
pub fn toggle_developer_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    if !opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "developer_panel") {
        return;
    }
    let on = api.text("advanced.developer_panel") == "true";
    let next = if on { "false" } else { "true" };
    if let Err(e) = api.change(
        "advanced.developer_panel",
        Some(next),
        crate::settings_registry::ChangeSource::Page,
        true,
    ) {
        tracing::warn!("developer panel setting: {e}");
    }
}

/// Shift + F10 switches the developer panel between its short and its detailed page.
pub fn toggle_developer_detail(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut detail: ResMut<super::overlay_info::DeveloperDetail>,
) {
    if opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "developer_panel_detail") {
        detail.full = !detail.full;
    }
}

/// The long page (Shift+F10): load counters, the camera and vehicle positions and asset problems.
fn details_text(stats: &WorldStats) -> String {
    format!(
        "[F10] map: {} (start tile {:?})\n{}\n{}\ntiles loaded: {} (missing {})\nscenery objects: {} (trees {}, protected-skipped {}, failed {}, no mesh {}) | splines: {} (failed {})\nvehicle: {} ({} meshes, {} verts)\nscene vertices: {} | asset failures: {} | load problems: {}\nkeyed meshes decoded exactly: {}",
        stats.map_name,
        stats.start_tile,
        format_pos("camera ", stats.position.camera, &stats.grid),
        stats.position.vehicle.map_or_else(
            || "vehicle: N/A".to_string(),
            |(p, h)| format!("{} heading {h:.0}°", format_pos("vehicle", Some(p), &stats.grid)),
        ),
        stats.tiles_loaded,
        stats.tiles_missing,
        stats.objects_loaded,
        stats.trees_loaded,
        stats.objects_protected,
        stats.objects_failed,
        stats.objects_no_geometry,
        stats.splines_loaded,
        stats.splines_failed,
        if stats.vehicle_name.is_empty() { "N/A" } else { &stats.vehicle_name },
        stats.vehicle_meshes,
        stats.vehicle_vertices,
        stats.scene_vertices,
        stats.asset_failures,
        stats.load_problems,
        stats.meshes_exact,
    )
}

/// The optional session facts the developer panel shows besides the world statistics.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PanelSources<'w, 's> {
    control: Option<Res<'w, opts_bevy::control::state::ControlState>>,
    environment: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    traffic: Option<Res<'w, super::traffic::TrafficState>>,
    scheduled: Option<Res<'w, super::traffic::ScheduledTraffic>>,
    stops: Query<'w, 's, (&'static GlobalTransform, &'static opts_bevy::bus_stops::BusStopMarker)>,
}

/// Refreshes the developer panel text while it is visible.
pub fn update_debug_overlay(
    stats: Res<WorldStats>,
    detail: Res<super::overlay_info::DeveloperDetail>,
    vehicle: Query<
        (&opts_bevy::VehicleController, &GlobalTransform),
        With<opts_bevy::PlayerVehicle>,
    >,
    sources: PanelSources,
    mut query: Query<(&mut Text, &mut Visibility), With<DebugOverlayText>>,
) {
    let PanelSources {
        control,
        environment,
        traffic,
        scheduled,
        stops,
    } = sources;
    for (mut text, mut vis) in &mut query {
        *vis = if stats.overlay_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !stats.overlay_visible {
            continue;
        }
        let facts = super::overlay_info::PanelFacts {
            fps: control.as_ref().map(|c| {
                let worst = c.frame_log.iter().map(|(_, ms)| *ms).fold(0.0, f32::max);
                (c.fps, worst)
            }),
            clock: environment.as_ref().map(|e| e.clock_text()),
            vehicle: stats.position.vehicle.map(|(p, h)| {
                let abs = stats.grid.absolute(p.tile, p.local.as_dvec2());
                ([abs.x, abs.y], p.world.z, h, speed_kmh(&vehicle))
            }),
            session: vehicle
                .single()
                .ok()
                .map(|(_, t)| [t.translation().x, -t.translation().z]),
            traffic: traffic.as_deref(),
            scheduled: scheduled.as_deref(),
            stops: stops
                .iter()
                .map(|(t, m)| (m.id_code, [t.translation().x, -t.translation().z]))
                .collect(),
        };
        text.0 = format!(
            "[F10 off, Shift+F10 {}] map: {}\n{}",
            if detail.full { "short page" } else { "details" },
            stats.map_name,
            super::overlay_info::lines(&facts)
        );
        text.0.push('\n');
        text.0
            .push_str(&format_pos("camera ", stats.position.camera, &stats.grid));
        if let Some((p, _)) = stats.position.vehicle {
            text.0.push('\n');
            text.0
                .push_str(&format_pos("vehicle", Some(p), &stats.grid));
        }
        if !detail.full {
            continue;
        }
        text.0.push('\n');
        text.0.push_str(&details_text(&stats));
        if let Ok((c, _)) = vehicle.single() {
            text.0.push('\n');
            text.0
                .push_str(&crate::control_dynamics::dynamics_text(&c.sim));
        }
    }
}

/// The player's vehicle speed (km/h), 0 without one.
fn speed_kmh(
    vehicle: &Query<
        (&opts_bevy::VehicleController, &GlobalTransform),
        With<opts_bevy::PlayerVehicle>,
    >,
) -> f32 {
    vehicle
        .single()
        .map_or(0.0, |(c, _)| c.sim.chassis.forward_speed_km_h().abs())
}

fn format_pos(label: &str, p: Option<WorldPos>, grid: &TileGrid) -> String {
    match p {
        Some(p) => {
            // Absolute coordinates are millions of metres: f32 keeps only half a metre, so the
            // panel computes them again from the tile and the local place in f64.
            let abs = grid.absolute(p.tile, p.local.as_dvec2());
            format!(
                "{label} tile {:?} local ({:.1}, {:.1}) | world ({:.1}, {:.1}, {:.1})",
                p.tile, p.local.x, p.local.y, abs.x, abs.y, p.world.z
            )
        }
        None => format!("{label} N/A"),
    }
}

/// Converts a Bevy-space position (x east, y up, z = −north, relative to the start tile's corner)
/// into an OMSI-frame position.
fn world_pos(grid: &TileGrid, start_tile: (i32, i32), p: Vec3) -> WorldPos {
    let (tile, local) = grid.tile_at(Vec2::new(p.x, -p.z).as_dvec2(), start_tile);
    let abs = grid.absolute(tile, local);
    WorldPos {
        tile,
        world: Vec3::new(abs.x as f32, abs.y as f32, p.y),
        local: local.as_vec2(),
    }
}

/// Tiles the camera and the vehicle were last logged in.
type LastTiles = (Option<(i32, i32)>, Option<(i32, i32)>);

/// Keeps [`WorldStats::position`] current and logs where the player is: `DEBUG` whenever the
/// camera or vehicle enters another tile, `TRACE` every 5 seconds, `INFO` when the session ends.
pub fn track_position(
    time: Res<Time>,
    mut since_trace: Local<f32>,
    mut last: Local<LastTiles>,
    mut stats: ResMut<WorldStats>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    vehicle: Query<&GlobalTransform, With<opts_bevy::PlayerVehicle>>,
    mut ended: Local<Option<PositionReadout>>,
) {
    let (start, grid) = (stats.start_tile, stats.grid);
    let cam = camera
        .single()
        .ok()
        .map(|c| world_pos(&grid, start, c.translation()));
    let veh = vehicle.single().ok().map(|v| {
        let (yaw, _, _) = v.rotation().to_euler(EulerRot::YXZ);
        // Bevy yaw is counter-clockwise about +Y; OMSI headings are clockwise from north.
        (
            world_pos(&grid, start, v.translation()),
            (-yaw.to_degrees()).rem_euclid(360.0),
        )
    });
    let readout = PositionReadout {
        camera: cam,
        vehicle: veh,
    };
    if stats.position != readout {
        stats.position = readout;
    }
    *ended = Some(readout);
    opts_bevy::set_position_tag(veh.map(|(v, _)| v).or(cam).map_or_else(String::new, |p| {
        format!(
            "tile {:?} local ({:.0}, {:.0}) world ({:.0}, {:.0}, {:.0})",
            p.tile, p.local.x, p.local.y, p.world.x, p.world.y, p.world.z
        )
    }));
    if let Some(c) = cam {
        if last.0 != Some(c.tile) {
            tracing::debug!(tile = ?c.tile, local = ?c.local, world = ?c.world, "camera entered tile");
            last.0 = Some(c.tile);
        }
    }
    if let Some((v, h)) = veh {
        if last.1 != Some(v.tile) {
            tracing::debug!(tile = ?v.tile, local = ?v.local, world = ?v.world, heading = h, "vehicle entered tile");
            last.1 = Some(v.tile);
        }
    }
    *since_trace += time.delta_secs();
    if *since_trace >= 5.0 {
        *since_trace = 0.0;
        tracing::trace!(camera = ?cam, vehicle = ?veh, "position");
    }
}

/// Logs the last known position when the session is left.
pub fn log_final_position(stats: Res<WorldStats>) {
    tracing::info!(
        camera = ?stats.position.camera,
        vehicle = ?stats.position.vehicle,
        "session ended at"
    );
}

/// `--debug-wheel-vars`: show the wheel animation variables of the player's vehicle.
#[derive(Resource, Default, Clone, Copy)]
pub struct DebugWheelVars(pub bool);

/// Marker of the wheel-variable overlay text.
#[derive(Component)]
pub struct WheelVarsText;

/// Lists `wheel_rotation_<n>_L/R` and `wheel_rotationspeed_<n>_l/r` (rad, rpm) per axle: the
/// head's axles, then each trailer's under the consist-wide numbers its model uses.
pub fn update_wheel_vars_overlay(
    mut commands: Commands,
    flag: Option<Res<DebugWheelVars>>,
    vehicle: Query<
        (
            &opts_bevy::VehicleController,
            &opts_bevy::VehicleScripts,
            Option<&opts_bevy::ArticulatedChain>,
        ),
        With<opts_bevy::PlayerVehicle>,
    >,
    segments: Query<&opts_bevy::ArticulatedSegment>,
    mut text: Query<&mut Text, With<WheelVarsText>>,
) {
    if !flag.is_some_and(|f| f.0) {
        return;
    }
    let Ok((controller, scripts, chain)) = vehicle.single() else {
        return;
    };
    let Ok(engine) = scripts.engine.lock() else {
        return;
    };
    let mut out = String::from("wheel variables (rad | rpm)\n");
    let line = |label: &str, n: usize| -> String {
        format!(
            "{label} axle {n}: rot L {:.2} R {:.2} | speed l {:.1} r {:.1}\n",
            engine.get(&format!("wheel_rotation_{n}_l")),
            engine.get(&format!("wheel_rotation_{n}_r")),
            engine.get(&format!("wheel_rotationspeed_{n}_l")),
            engine.get(&format!("wheel_rotationspeed_{n}_r")),
        )
    };
    for n in 0..controller.sim.axles.len() {
        out.push_str(&line("head", n));
    }
    if let Some(chain) = chain {
        for (k, &e) in chain.segments.iter().enumerate() {
            out.push_str(&format!(
                "joint {k}: alpha {:.2} beta {:.2} gamma {:.2} deg\n",
                engine.get(&format!("articulation_{k}_alpha")),
                engine.get(&format!("articulation_{k}_beta")),
                engine.get(&format!("articulation_{k}_gamma")),
            ));
            if let Ok(seg) = segments.get(e) {
                for j in 0..seg.wheels.len() {
                    out.push_str(&line(&format!("trailer {}", k + 1), seg.first_axle + j));
                }
            }
        }
    }
    if let Ok(mut t) = text.single_mut() {
        t.0 = out;
    } else {
        commands.spawn((
            Text::new(out),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.95, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(70.0),
                right: Val::Px(12.0),
                ..default()
            },
            WheelVarsText,
            InGameEntity,
        ));
    }
}
