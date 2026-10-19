//! Jump to an entry of the route.

use super::*;

/// *Jump to entry point*: the camera goes to the entry (free flight, looking along it) and the
/// active vehicle is put there, if there is one.
pub(in super::super::super) fn jump_to_entry(world: &mut World, id: u32) {
    use crate::session_vehicles::SpawnPoint;
    let found = {
        let (Some(job), Some(index)) = (
            world.get_resource::<WorldLoadJob>(),
            world.get_resource::<EntryIndex>(),
        ) else {
            return;
        };
        index.find(id).map(|e| {
            let session = job.session_pos(e.tile, e.local);
            let abs = job.grid().absolute(e.tile, e.local.truncate().as_dvec2());
            (session, abs, e.local.z, e.heading_deg, e.name.clone())
        })
    };
    let Some((session, abs, z, heading, name)) = found else {
        set_status(
            world,
            "That entry point is not located yet (or sits on a spline).".into(),
        );
        return;
    };
    // The vehicle first, then the camera, so the camera looks at the vehicle from behind.
    let active = world.resource::<SessionVehicles>().active_id();
    if let Some(vid) = active {
        let point = SpawnPoint {
            x: abs.x,
            y: abs.y,
            z: f64::from(z),
            heading_deg: f64::from(heading),
        };
        if let Err(e) = ops::respawn_at(world, vid, point) {
            tracing::warn!("jump: vehicle not moved: {e}");
        }
    }
    let at = opts_bevy::coords::omsi_to_bevy_pos(session);
    let yaw = -heading.to_radians();
    let pitch = -0.2_f32;
    let rot = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    let eye = at + Vec3::Y * 4.0 + rot * Vec3::Z * 12.0;
    if let Some(mut state) = world.get_resource_mut::<opts_bevy::CameraState>() {
        state.switch_to(opts_bevy::CameraMode::FreeRoam);
        state.free_roam_spawn = eye;
        (state.free_roam_spawn_yaw, state.free_roam_spawn_pitch) = (yaw, pitch);
        (state.free_roam_yaw, state.free_roam_pitch) = (yaw, pitch);
    }
    let mut cams = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
    for mut tf in cams.iter_mut(world) {
        *tf = Transform::from_translation(eye).with_rotation(rot);
    }
    tracing::info!(entry = %name, id, "jumped to entry point");
}

pub(in super::super::super) fn build_jump(
    commands: &mut Commands,
    menu: &VehicleMenu,
    entries: &[(u32, String)],
    resolved: bool,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 80.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(content).with_children(|p| {
        label(p, "Jump to entry point", 22.0, ColorRole::Primary);
        label(
            p,
            if resolved {
                "The camera goes there; the active vehicle is put there too."
            } else {
                "Entry points are still being located; some may not work yet."
            },
            13.0,
            ColorRole::OnSurfaceVariant,
        );
        if !menu.status.is_empty() {
            label(p, menu.status.clone(), 13.0, ColorRole::OnSurfaceVariant);
        }
        scroll_list(p, 2, menu.scroll[2], Val::Percent(100.0), |list| {
            let mut seen: HashMap<&str, usize> = HashMap::new();
            let mut total: HashMap<&str, usize> = HashMap::new();
            for (_, n) in entries {
                *total.entry(n.as_str()).or_default() += 1;
            }
            for (id, name) in entries {
                let n = seen.entry(name.as_str()).or_default();
                *n += 1;
                let text = if total[name.as_str()] > 1 {
                    format!("{name} ({n})")
                } else {
                    name.clone()
                };
                let b = button(list, &text, false, Action::JumpEntry(*id));
                list.commands()
                    .entity(b)
                    .insert(UiId::new(format!("vehicle.jump.{id}")));
            }
        });
        button(p, "Back", false, Action::Back);
    });
}
