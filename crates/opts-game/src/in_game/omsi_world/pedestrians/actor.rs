//! Characters as actors for other systems (the passengers): spawning one and posing it.

use super::*;

/// The gait phase after `dt` seconds of walking at `speed` (m/s).
pub(crate) fn advance_phase(human: &Human, phase: f32, speed: f32, dt: f32) -> f32 {
    let stride = human.profile.walk.stride_m.max(0.3);
    (phase + std::f32::consts::TAU * speed / stride * dt) % std::f32::consts::TAU
}

/// Poses the joints of a standing character in the walking gait at `phase`.
pub(crate) fn apply_gait(
    human: &Human,
    leg_m: f32,
    phase: f32,
    transforms: &mut Query<&mut Transform>,
) {
    let g = gait(&human.profile.walk, leg_m, phase);
    let mut q = pose_rotations(&human.profile, HumanPose::Standing);
    q[3] = Quat::from_rotation_x(g.thigh[0]);
    q[4] = Quat::from_rotation_x(g.thigh[1]);
    q[5] = Quat::from_rotation_x(-g.knee[0]);
    q[6] = Quat::from_rotation_x(-g.knee[1]);
    q[7] = Quat::from_rotation_x(g.arm[0]) * q[7];
    q[8] = Quat::from_rotation_x(g.arm[1]) * q[8];
    q[1] = Quat::from_rotation_y(g.waist_twist);
    for (i, joint) in human.rig.joints.iter().enumerate() {
        if let Ok(mut t) = transforms.get_mut(*joint) {
            t.rotation = q[i];
            if i == 0 {
                let rest = rest_pivots(&human.profile)[0];
                t.translation = rest + Vec3::new(g.hip_shift, -g.bob, 0.0);
            }
        }
    }
}

/// Poses a character at rest: standing, or seated.
pub(crate) fn apply_rest_pose(
    human: &Human,
    pose: HumanPose,
    transforms: &mut Query<&mut Transform>,
) {
    let q = pose_rotations(&human.profile, pose);
    for (i, joint) in human.rig.joints.iter().enumerate() {
        if let Ok(mut t) = transforms.get_mut(*joint) {
            t.rotation = q[i];
            if i == 0 {
                t.translation = rest_pivots(&human.profile)[0];
            }
        }
    }
}

/// A character of the map for a passenger.
pub(crate) struct Actor {
    /// The root entity of the rig.
    pub entity: Entity,
    /// Leg length for the gait (m).
    pub leg_m: f32,
    /// Age from the profile, when it states one.
    pub age: Option<u32>,
}

impl PedestrianPool {
    /// Asks for the characters to be loaded (passengers).
    pub(crate) fn want(&mut self) {
        self.wanted = true;
    }

    /// Spawns a character drawn by the `humans.txt` weights with the value `unit` in `[0, 1)`,
    /// standing; `None` while no character has finished loading.
    pub(crate) fn spawn_actor(
        &self,
        commands: &mut Commands,
        bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
        unit: f32,
    ) -> Option<Actor> {
        let loaded: Vec<usize> = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, (_, v, _))| v.is_some())
            .map(|(i, _)| i)
            .collect();
        if loaded.is_empty() {
            return None;
        }
        let weights: Vec<usize> = loaded
            .iter()
            .map(|&i| {
                self.entries
                    .iter()
                    .filter(|e| e.eq_ignore_ascii_case(&self.slots[i].0))
                    .count()
                    .max(1)
            })
            .collect();
        let mut pick = (unit.clamp(0.0, 0.999) * weights.iter().sum::<usize>() as f32) as usize;
        let mut chosen = loaded[0];
        for (k, w) in weights.iter().enumerate() {
            if pick < *w {
                chosen = loaded[k];
                break;
            }
            pick -= w;
        }
        let v = self.slots[chosen].1.as_ref()?;
        let leg_m = rest_pivots(&v.profile)[0].y.max(0.3);
        let entity = spawn_human(
            commands,
            bindposes,
            &v.profile,
            &v.parts,
            &v.name,
            HumanPose::Standing,
        );
        commands
            .entity(entity)
            .insert((Visibility::default(), InGameEntity));
        Some(Actor {
            entity,
            leg_m,
            age: v.profile.age_years,
        })
    }
}
