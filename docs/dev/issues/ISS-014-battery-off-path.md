# ISS-014: Battery-off path untested

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-014 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0160](../../src/roadmap/phase-17/M-0160.md) Battery-off path test and cockpit script check; the card name in [M-0123](../../src/roadmap/phase-12/M-0123.md) is corrected under this issue |
| **Feature audit** | FEAT-08 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | in progress 2026-11-22 on `agent/battery-off-test` (from master `ecefa84b`). Open: the script-rig test and its removal check; the parser result for the four HH109 `cockpit.osc` files; the M-0123 name fix. |

## Problem

Turning the battery switch off must make every electrical output zero. Nothing tests that path. Two open questions:

- The battery switch's off press is not covered by any test. The game raises `cp_batterietrennschalter_toggle_off` on release (the held key action in `crates/opts-game/src/vehicle_keys/table.rs`), and the code also produces `kw_batterietrennschalter_off` through the scripts (cockpit `{trigger:kw_batterietrennschalter_off}`).
- The M-0123 card names `cp_batterietrennschalter_off`, which matches nothing in code.
- The content issue registry lists an `{if}` without `{endif}` in the battery toggle trigger of four HH109 `cockpit.osc` files (registry lines 921, 938, 950, 966). That is a registry result from a corpus run, not yet re-checked on this branch.

## Facts read from the scripts (logic only, in prose)

The HH109 Hamburg cockpit scripts in the OMSI installation (read-only, not copied):

- The battery toggle trigger sets `elec_busbar_main_sw` (the switch) in its main branch and clears it in its `{else}` branch. It also sets `cp_taster_batterietrennschalter` (the key's visual state) and `cp_schluessel_trans`.
- `elec.osc` makes the main busbar `elec_busbar_main` active only when the bus is available, `elec_busbar_main_sw` is set and there is no electrical failure. Electrical outputs (lights, EFAD and others) read `elec_busbar_main_sw` or `elec_busbar_main`.
- The `_off` trigger for the toggle only clears `cp_batterietrennschalter_antirepeat` and `cp_taster_batterietrennschalter`; it does not touch the switch.

Parser check: see the section below, filled in when the check is run.

## Work

1. Script-rig test on an in-code fixture: press the battery toggle, release it, press again to switch off, release; assert `elec_busbar_main` is zero when the switch is off. Must fail when the off branch is removed.
2. Run the project parser (`ScriptEngine::from_sources`) over the four HH109 `cockpit.osc` files and record the diagnostic for the battery trigger.
3. Correct the M-0123 name to `cp_batterietrennschalter_toggle_off` with the evidence (the `_off` release and the `kw_batterietrennschalter_off` script trigger).

The HH109 game files are not edited. The result goes to the owner.

## Parser result

Pending.
