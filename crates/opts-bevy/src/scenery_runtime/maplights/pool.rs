//! The pool of point lights: slot bookkeeping, growth and the assignment of lamps to slots.

use bevy::prelude::*;

/// One light of the pool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolSlot {
    /// The pool's point light entity.
    pub light: Entity,
    /// The lamp it shows (`None`: free).
    pub target: Option<Entity>,
    /// The lamp is still chosen (otherwise the slot fades out and becomes free).
    pub active: bool,
    /// Fade level 0–1.
    pub level: f32,
}

/// The pool of point lights shared by all `[maplight]`s.
#[derive(Resource, Debug, Default, Clone)]
pub struct MapLightPool {
    /// The pool's lights (spawned on first use).
    pub slots: Vec<PoolSlot>,
    /// Seconds since the last assignment.
    pub since_assign_s: f32,
}

impl MapLightPool {
    /// Lamps that currently hold a light (fading out ones included).
    #[must_use]
    pub fn assigned(&self) -> Vec<Entity> {
        self.slots.iter().filter_map(|s| s.target).collect()
    }
}

pub(super) fn ensure_pool_capacity(commands: &mut Commands, pool: &mut MapLightPool, size: usize) -> bool {
    if pool.slots.len() < size {
        for _ in pool.slots.len()..size {
            let light = commands
                .spawn((
                    PointLight {
                        intensity: 0.0,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::default(),
                    Visibility::Hidden,
                    Name::new("MapLightPool"),
                ))
                .id();
            pool.slots.push(PoolSlot {
                light,
                target: None,
                active: false,
                level: 0.0,
            });
        }
        true
    } else {
        false
    }
}

/// Keeps slots whose lamp is still chosen, lets the others fade out and gives free slots to the
/// chosen lamps that have none.
pub fn assign(slots: &mut [PoolSlot], chosen: &[Entity]) {
    for slot in slots.iter_mut() {
        slot.active = slot.target.is_some_and(|t| chosen.contains(&t));
    }
    let held: Vec<Entity> = slots.iter().filter_map(|s| s.target).collect();
    let mut waiting = chosen.iter().filter(|e| !held.contains(e));
    for slot in slots.iter_mut().filter(|s| s.target.is_none()) {
        let Some(&next) = waiting.next() else {
            break;
        };
        *slot = PoolSlot {
            light: slot.light,
            target: Some(next),
            active: true,
            level: 0.0,
        };
    }
}
