# ISS-049: Gamepad, wheel and driver's desk input

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-049. Research item RG-08 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [M-0035](../../src/roadmap/phase-7/M-0035.md) for gamepad and wheel (owner decision 2026-11-22; [M-0142](../../src/roadmap/phase-7/M-0142.md) status line now agrees); [M-0168](../../src/roadmap/phase-18/M-0168.md) Driver's Desk Hardware Interface (proposed 2026-11-22) for the desk |
| **Status** | owner decision 2026-11-22: gamepad and wheel in M-0035; desk interface in scope, milestone M-0168 (not approved to start) |

## Problem

OMSI 2 lists steering-wheel and joystick axis settings, with a wheel setup guide. LOTUS lists keyboard and controller settings and a plugin interface for a driver's desk.

OPTS state:

- Keyboard rebinding has a first slice. The registry has 26 built-in actions, and the key list is in `crates/opts-game/src/vehicle_keys.rs` and the help modules ([M-0142](../../src/roadmap/phase-7/M-0142.md)).
- M-0142's status line says gamepad and wheel rebinding is still open. Its "Out of scope" section says the gamepad and steering wheel mapping needs [M-0035](../../src/roadmap/phase-7/M-0035.md)'s device work. These two statements disagree and need one answer.
- The analogue axes for throttle and steering are not rebindable. M-0142 names them as M-0035's.
- There is no device plugin interface for a driver's desk. The control protocol ([ADR-0006](../../src/adr/0006-control-protocol.md)) is for tooling and is not a device interface.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Device support is specified from the public documentation of the input APIs and from OPTS's own tests. No OMSI or LOTUS input code or configuration files are used.

## Work

1. Resolve the M-0142 contradiction: either gamepad and wheel are in M-0142's scope, or they belong to M-0035 and the status line is corrected. Check that M-0035 owns the analogue axes.
2. Ask the owner whether a driver's desk interface is in scope at all. If it is out, record that. If it is in, it needs its own design note, and it should be decided together with ISS-045 (TrackIR) because both are device interfaces.
3. If in scope, open a proposed milestone for the desk interface and link it here.

## Done when

- M-0142's status and its out-of-scope section say the same thing about gamepad and wheel.
- The desk interface has either a recorded decision (out of scope) or a proposed milestone.
