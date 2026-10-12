# Chain assembly: seeding followers and position jumps (P-1011)

`crates/opts-bevy/src/articulation.rs`, `articulation_system`.

## Problem
A following segment (`ArticulatedSegment`) created its kinematic state on the first fixed step from the
leader's pose. The game places the vehicle afterwards: in the Aachen test the head moved about 4 m over steps 2-8
(0.5 m per step at velocity 0). The follower integrated that displacement as motion and swung to the joint limit
(alpha -52.5 degrees), then unwound over about 4 s. Symptoms of this one cause: the jackknifed start, folded
bellows, the trailer wheels turning slower than the head's while unwinding (ratio 0.63 at 3.6 km/h), and wheel speed
spikes whenever the head jumped later (collision push-out, reset).

## Rule
One detection, one response:

- **Detection:** a leader position change in one step larger than `max(|v|*dt*2, 0.3 m)` is a position jump, not
  motion. The threshold (`JUMP_THRESHOLD_M`) is a heuristic: it is above any per-step displacement the simulation
  produces at any speed (the first term scales with speed) and below the 0.5 m steps of the spawn placement.
  The first step of a chain counts as a jump (nothing to compare with).
- **Response:** the segments are re-seeded behind the leader's current pose (joint angle 0), interpolation
  poses are reset, and the segments' wheel variables keep their last values for that step (no spin from the jump).

## Tests (`crates/opts-bevy/tests/articulation.rs`)
`spawn_displacement_of_the_leader_does_not_jackknife_the_segment` (alpha/beta/gamma < 0.01 after 10 steps) and
`a_leader_jump_does_not_spin_the_segment_wheels` (rpm never above 1.5x plausible, alpha < 5 after a 3.6 m jump).
Both failed before the fix (alpha 38.2 and 28.3 degrees).

Note: segment mesh counts may be zero on the first frame (async decode of scrambled meshes on a cold cache); this is
expected and does not affect gameplay.
