//! Per-slot update of a pooled point light: fade, colour, intensity and visibility.

use bevy::prelude::*;

use super::component::MapLight;
use super::photometry::{map_light_lumens, map_light_range};
use super::pool::PoolSlot;
use super::settings::MapLightSettings;

pub(super) fn update_slot_light(
    slot: &mut PoolSlot,
    lamp: Option<(Entity, &MapLight, &GlobalTransform)>,
    lights: &mut Query<(&mut PointLight, &mut Transform, &mut Visibility), Without<MapLight>>,
    step: f32,
    night: bool,
    scale: f32,
    settings: &MapLightSettings,
) {
    let want = if slot.active && night && lamp.is_some() {
        1.0
    } else {
        0.0
    };
    slot.level = if want > slot.level {
        (slot.level + step).min(want)
    } else {
        (slot.level - step).max(want)
    };
    if slot.level <= 0.0 && !slot.active {
        slot.target = None;
    }
    let Ok((mut point, mut transform, mut visibility)) = lights.get_mut(slot.light) else {
        return;
    };
    match lamp.filter(|_| slot.level > 0.0) {
        Some((_, m, t)) => {
            point.color = Color::srgb(m.color.x, m.color.y, m.color.z);
            point.intensity = map_light_lumens(m.radius_m, settings.full_lux) * slot.level * scale;
            point.range = map_light_range(m.radius_m);
            transform.translation = t.translation();
            if *visibility != Visibility::Inherited {
                *visibility = Visibility::Inherited;
            }
        }
        None => {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
                point.intensity = 0.0;
            }
        }
    }
}
