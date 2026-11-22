# ISS-014: Battery-off path untested

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-014 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0160](../../src/roadmap/phase-17/M-0160.md) Battery-off path test and cockpit script check |
| **Feature audit** | FEAT-08 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | Rig test done 2026-11-22 at `9e29cfb9` (removal check FAILED on the off branch, file restored). Open for the owner: the four HH109 `cockpit.osc` files have an unclosed `{if}` in the battery toggle trigger (parser check below). Nothing in the HH109 game files was edited. |

## Problem

Turning the battery switch off must make every electrical output zero. Nothing tested that path. Three open questions:

- The battery switch's off press was not covered by any test. The game raises `cp_batterietrennschalter_toggle_off` on release (the held key action `cp_batterietrennschalter_toggle` in `crates/opts-game/src/vehicle_keys/table.rs`; the `_off` name is built in `crates/opts-game/src/control_drive/drive_set.rs`), and the scripts react to `kw_batterietrennschalter_off`.
- The M-0123 card was read as naming `cp_batterietrennschalter_off`. Checked 2026-11-22: the card names no battery trigger at all. The name appears only in the audit register and in this issue's intake (commits `29513a8c`, `d50b1cad`, `786a4572`).
- The content issue registry lists an unclosed `{if}` in the battery toggle trigger of four HH109 `cockpit.osc` files (registry lines 921, 938, 950, 966). Re-checked on this branch with the parser: confirmed (below).

## Facts read from the scripts (logic only, in prose)

Read from the OMSI 2 install, HH109_Stadtbus_HHA `cockpit.osc` and `elec.osc` (read-only, nothing copied):

- The toggle trigger (lines 836 to 875) has an `{if}` at line 839 that tests the key's transition mode. Its main branch sets the switch `elec_busbar_main_sw` to 1 (line 852, with the key visual `cp_taster_batterietrennschalter`, the antirepeat flag and `cp_schluessel_trans`). Its `{else}` (line 858), which is the off press, sets `elec_busbar_main_sw` to 0 (line 869).
- The `{if}` at line 839 has no `{endif}`: the `{else}` at line 858 closes it, and the parser closes it at `{end}` (see the parser result).
- The release trigger `cp_batterietrennschalter_toggle_off` (line 877) clears `cp_batterietrennschalter_antirepeat` and `cp_taster_batterietrennschalter` only. It does not touch the switch.
- The key-visual triggers `kw_batterietrennschalter` (line 813) and `kw_batterietrennschalter_off` (line 830) set and clear `cp_taster_batterietrennschalter` only.
- `elec.osc` sets the main busbar `elec_busbar_main` when `elec_busbar_avail` and `elec_busbar_main_sw` are true and `elec_failure_general` is false (lines 84 to 87).
- Electrical outputs that read the switch directly: the front, rear and driver lights and EFAD, and the battery voltage drain (`elec.osc` line 46). Further readers of the busbar or switch: `door.osc` (lines 236, 860, 1442) and `AI.osc` (line 23).

## Work

1. Script-rig test on an in-code stand-in: press the battery toggle, release it, press again to switch off, release; assert `elec_busbar_main` is zero when the switch is off. Done: `9e29cfb9`.
2. Project parser (`ScriptEngine::from_sources`) over the four HH109 `cockpit.osc` files. Done (below).
3. Correct the M-0123 notes with the real trigger names. Done (M-0123 card note, see the outcome).

The HH109 game files are not edited. The result goes to the owner.

## Parser result

Run 2026-11-22 on `agent/battery-off-2` with the project's `ScriptEngine::from_sources`, one file at a time, on the OMSI 2 install (`Vehicles/<vehicle>/Script/cockpit.osc`, read-only). Each file gives the warning `{if} without {endif} in trigger:cp_batterietrennschalter_toggle closed at {end}`:

| Vehicle folder | Parser line of the `{if}` | Trigger line in file | Total diagnostics |
| :--- | :--- | :--- | :--- |
| HH109_Stadtbus_HHA | 839 | 836 | 8 |
| HH109_Stadtbus_HHA_AI | 814 | 811 | 9 |
| HH109_Stadtbus_VHH | 814 | 811 | 7 |
| HH109_Stadtbus_VHHPVG_AI | 814 | 811 | 8 |

The same files also carry other diagnostics (unmatched `{endif}` and `{end}`, and an unclosed `{if}` in a macro) that this issue did not examine. Not checked here: whether the engine runs the `{else}` as the off branch of this unclosed `{if}`. The rig test uses a balanced stand-in, so it does not answer that.

## Outcome

- **Rig test: done** at `9e29cfb9`. Test `vehicle_setup::tests::battery_off::releasing_the_battery_switch_off_takes_the_busbar_to_zero` on an in-code stand-in in the shape of the switch logic above (no game script copied). The stand-in does not model the availability and failure terms of `elec.osc`.
- **Removal check: FAILED as required.** The stand-in's off branch was removed and the test was run: `test vehicle_setup::tests::battery_off::releasing_the_battery_switch_off_takes_the_busbar_to_zero ... FAILED` (`switch off`, left 1.0, right 0.0). The file was restored with `git checkout` and the test passes again: `1 passed; 0 failed`.
- **M-0123 name: corrected.** The card never named `cp_batterietrennschalter_off`. The M-0123 card now lists the real names in a note, and the wrong name is recorded as a misreading.
- **Open for the owner: the four HH109 `cockpit.osc` files.** The battery toggle trigger has an unclosed `{if}` (parser lines above). The game files are not in this repository and were not edited. Until the owner fixes or accepts that, the real script's off branch is not covered by any test.
