# M-0123 open decisions (research, 2026-11-24)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0123](../../src/roadmap/phase-12/M-0123.md) Fuel and Battery State |
| **Basis** | `master` at `2290ec0c` |
| **Method** | Text scripts only (`.osc`, varlists, constfiles, `.bus`) read from the OMSI 2 install (paths below are relative to its `Vehicles/` folder), the repo's format docs and repo code. No executable or DLL was inspected ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Script text is paraphrased, not copied. |
| **Status** | Research only. No code, no BOARD or ROADMAP change. |

## Summary

| # | Decision | Recommended option |
| :--- | :--- | :--- |
| 1 | Authority for `tank_percent` | B: the script owns the level in the four stock families and the families that write it; the host mirrors it. The feed scale changes from 0 to 100 to the stock 0 to 1 fraction. |
| 2 | ISS-042, the electric variable | Record both variable families on the card and the fuel page; move the electric state of charge to a follow-up item. |
| 3 | Full-tank default for saves without `fuel` | C: take the level from the script's saved tank content when the vehicle has one; otherwise a full tank. |
| 4 | `[petrolstation]` | A for M-0123: F12 when stopped stays the only refuel. Stations go to a follow-up milestone. |

---

## 1. Authority for `tank_percent`

### Facts

- **The stock engine scripts write it.** Each writes `tank_percent` once per frame, as the engine's tank content divided by a capacity literal:
  - `MAN_NL_NG/Script/engine.osc:274` and `engine_GN.osc:275` (capacity 250)
  - `MAN_SD200/Script/engine.osc:273` and `engine_X10_AI.osc:117` (250)
  - `EMT_Citaro/Script/engine.osc:312` (265)
  - `Urbino_II/script/Urbino_II_Engi.osc:262` (250)

  The capacity is a literal inside each script, not a constant-file value. The result is a fraction from 0 to 1.
- **The stock reader expects a fraction.** Only the Citaro cockpit reads it: `EMT_Citaro/Script/vdv_cockpit.osc:3294` draws the fuel bar as `tank_percent × 48` cells, and `:3719` flags low fuel below `0.1`. `vdv_cockpit_solo.osc:3285` and `:3705` are the same. The stock scale is therefore 0 to 1, not 0 to 100.
- **OPTS feeds the other scale.** `HostInputs::tank_percent` is level × 100 (`docs/src/simulation/fuel-battery.md` §5; M-0123 acceptance criterion 3 says "level × 100"). A host value only reaches a script that does not write the variable, so for the four stock families it is overwritten each frame. The mismatch matters for any reader whose vehicle has no writer.
- **The stock engine already stops at empty.** When the content is zero or below, it sets `engine_failure_general` (`MAN_NL_NG/Script/engine.osc:143-144`, `MAN_SD200/Script/engine.osc:141-142`, `Urbino_II_Engi.osc:185-186`, `EMT_Citaro/Script/engine.osc:~417`). The host torque cut is therefore a second cut for these families.
- **The stock drain is the script's own.** Each frame the content falls by the engine's consumption rate times the frame time, divided by 3600 (`MAN_NL_NG/Script/engine.osc:~432`). The scripts do not name the unit. Searches for "litre" and "liter" in the engine scripts found nothing, so the unit is still ❓. The 250 or 265 literal suggests litres.
- **Refuelling is the stock `veh_tank` trigger.** It adds three times the frame time to the content on each call, capped at the capacity (`MAN_NL_NG/Script/engine.osc:129`; Citaro `:43-44` with 265). A held trigger refills at about 3 units per second, so a 250 tank takes about 83 s. Nothing in the four stock families' non-script files raises `veh_tank` (a grep of their `.cfg`, `.bus`, `.sco` and text files found none). The raiser is ❓: the host or the engine.
- **The stock start is random.** `engine_init` sets the content to 50 to 250 (Citaro 65 to 265), not a full tank.
- **The save already holds the stock content.** The vehicles section saves every non-zero script float (`crates/opts-game/src/session_vehicles/state.rs`, `script_floats`), which includes `engine_tank_content` and `tank_percent`.
- **Two stores exist.** The host model (throttle-driven, OPTS constants) and the script's content (its own consumption) drift apart whenever both are active.

### Options

| Option | What it means | Player impact | Save impact |
| :--- | :--- | :--- | :--- |
| A. Host authoritative | The host writes `engine_tank_content` each frame from its model. OPTS needs the capacity per family (250 or 265), which is a script literal, so it needs a table. Accelerated and Off work through the model. | One model for the player. The stock "stop at empty" still fires from the written value. | The `fuel` section is the truth; restore overwrites the script floats. |
| B. Script authoritative, host mirror | For families whose engine script writes the content: the host reads the level as content ÷ capacity and treats it as read-only. Accelerated subtracts an extra drain from the content. Off writes the content back to its start value. The host torque cut is not applied to these families. Families with no such script keep the host model. | The stock behaviour stays; Accelerated multiplies the stock drain; Off freezes it. | No new data for these families; their content is already in the vehicles section. |
| C. Two models (status quo) | Each store drains on its own. | Readings disagree; Accelerated and Off apply to one store only. | Two levels to restore. Rejected. |

### Recommendation: B

- It keeps one store, the script's variable. The stock scripts, the stock stop at empty and the existing vehicles save all use it.
- Option A has to guess the capacity per family (250 or 265) and overwrites the script's drain every frame. That fights the script instead of using it.
- Required changes if B is adopted:
  1. Feed `tank_percent` as the fraction (level, not level × 100). Correct M-0123 acceptance criterion 3 and `fuel-battery.md` §1 and §5.
  2. For families whose engine writes the content, mirror it and apply the Accelerated drain and the Off freeze. Do not apply the host torque cut to them.
  3. Keep the host model for families with no such script.
- Still open: the unit of the content (❓, probably litres) and who raises `veh_tank` (❓). Neither blocks B.

---

## 2. ISS-042: the electric variable

### Facts

- **The name is not in the predefined list.** `docs/src/formats/text/vehicles/predefined-variables.md` and `varnamelist.md` contain no battery name. The electric variables are declared in each vehicle's own varlist (for example `Bollore_Bluebus/Script/elec_varlist.txt`).
- **Family A: `elec_battery_load` (fraction).** Used by Bollore Bluebus (`Bollore_Bluebus/Script/elec.osc`) and by one file of the Heuliez X37 Vol2 set (`HeuliezBus_X37_Vol2/Script/Elec.osc`).
  - It starts at 1.0 (`elec.osc:34`) and falls by the bus-bar current times the frame time, divided by 3600 and by the current capacity `elec_battery_Q_curr` (`elec.osc:149`, `:154`).
  - Charging raises it with a cap (`:149`).
  - `elec_battery_Q_curr` is the nominal capacity (`elec_battery_Q` = 135, `Bollore_Bluebus/Script/elec_constfile.txt`) corrected for temperature and age (`elec.osc:116`).
  - So `elec_battery_load` is a state of charge from 0 to 1.
- **Family B: `electric_battery` (absolute units).** Used by `HH_EBus2019/Script/19_engine.osc` and by `HH20_EBus2021` (4 files).
  - The capacity constant is `electric_battery_capacity` = 250 (`HH_EBus2019/Script/19_engine_constfile.txt`).
  - The state of charge is `electric_battery / electric_battery_capacity` (`19_engine.osc:367`; the dashboard at `19_cockpit_C2_2T.osc:3875`).
  - The dashboard shows 11 segments, `_display_soc_1` to `_display_soc_11`.
  - The engine checks the battery against 0.01 in absolute units (`19_engine.osc:97`, `:258`, `:358`). What those branches do was not traced.
- **Other electric folders not resolved.** `HC_EBus2019` and `HH20_EBus2019` have no hit in their script folders; their scripts were not located.
- **The electric scripts already model the battery.** The bus-voltage model is in the scripts; M-0123 notes the bus-bar state as the existing signal.

### Options

| Option | What it means | Player impact | Save impact |
| :--- | :--- | :--- | :--- |
| A. Add the state of charge to M-0123 now | Host inputs for both families, with a table that maps the vehicle to its variable and rule. | The HUD shows the charge of electric buses. | A read-only input; no new section needed. |
| B. Record and defer | Write both names and the rules on the M-0123 card and `fuel-battery.md`; close the ISS-042 open item by moving the electric display to a follow-up item. The electric branch of the model stays unused for stock electric buses. | No change for the player. | None. |
| C. Use the script's own battery values | Drop the OPTS electric constants (2 and 120 units per hour, the 20 % limp fraction). Let the script's battery be the authority, as in item 1. | The stock battery behaviour applies; Accelerated would change the script's drain. | None. |

### Recommendation: B, with C as the follow-up design

- The two families use different variables, units and rules. A is a per-family table with two code paths.
- The stock electric scripts already run their own battery. A player's electric bus works without OPTS.
- The model's electric constants (`ELECTRIC_IDLE_UNITS_PER_H` and the limp fraction) are OPTS assumptions. Showing them as a charge would show a number the stock model does not use.
- The limp fraction would be a second, non-stock torque cut, so item 1's rule should apply to electric families as well.
- Close ISS-042 with the two names and the two rules, and file the follow-up (display and any authority decision) under a new item.

---

## 3. Full-tank default for saves without the `fuel` section

### Facts

- **The loader skips the section.** `plan_restore` (`crates/opts-game/src/save/registry/restore_plan.rs`) puts an absent section in `missing` and applies nothing for it. Restore is not blocked; only a missing map or missing vehicles block it (`crates/opts-game/src/save/restore.rs:33`).
- **What then applies.** `sync_fuel_mode` (`crates/opts-game/src/vehicle_fuel_settings.rs:50-70`) gives each vehicle without a model a diesel model of 250 at the start share. The share is `VehicleSetup::default().fuel` = 0.6 (`crates/opts-bevy/src/vehicle_setup.rs:77`) or the player's `vehicle.fuel` setting. So an old save gets 60 %, not a full tank.
- **The acceptance criterion says full.** M-0123 acceptance criterion 5 says "an old save without the section restores a full tank". The status row marks this as not met.
- **The vehicles section holds the stock content.** Restored script floats bring back the stock tank content (item 1), so for stock families the saved level is already in the save.
- **The stock start is not full either.** `engine_init` starts at 50 to 250 of 250 or 265 (item 1).
- **The `script_floats` list omits zeros.** An empty tank is not stored, so a missing entry can mean zero or "no such variable". The restore needs a marker to tell them apart.

### Options

| Option | What it means | Player impact | Save impact |
| :--- | :--- | :--- | :--- |
| A. Full tank | Level 1.0 when the section is absent (criterion 5 as written). | Old saves never strand a vehicle. For stock families the script's saved content then disagrees with the host level unless item 1 is done. | None. |
| B. Keep the start share (current) | Level 0.6 or the player's share. | Matches a new session, not the criterion. | None. |
| C. Script content first, full tank as fallback | When the vehicle has a stock content variable in its saved script floats, the level is content ÷ capacity. Otherwise level 1.0. Needs the marker for "variable exists" described above. | The vehicle keeps the fuel it had. | None; the `fuel` section stays v1. |

### Recommendation: C

- The saved script content is the real fuel of those vehicles, so an old save restores what the player had.
- A full tank is the right fallback when no record exists, and it is what criterion 5 asks.
- C follows from item 1, option B: the same content is the level for stock families.
- Implementation note: decide "variable exists" from the vehicle's declared variables, not from the saved floats alone, so an empty tank is not read as "no record".
- Amend criterion 5 to name the source: script content first, full tank otherwise.

---

## 4. `[petrolstation]`

### Facts

- **The format docs say:** it enables refuelling and washing near the object (`docs/src/formats/text/sco.md:612`; `docs/src/formats/community-crosscheck.md:102`). Both cite the community wiki (community-wiki fact, not verified in the corpus).
- **The audit lists it as a bare flag** that classifies the object, with ❓ on its effect (`docs/src/formats/text/sco-sli-audit.md:110`, 58 occurrences; "no mutation test in OMSI has been run").
- **The corpus has about 60 files.** A grep of `Sceneryobjects/` finds 60 `.sco` files (for example `Buildings_MC/tank_*.sco`, tank-station buildings).
- **OPTS ignores it.** No code names it. The `.sco` parser skips unknown keywords (`crates/opts-omsi-scenery/src/formats/scenery/sco/parse.rs:148`, `_ => i += 1`).
- **The link to refuelling is unverified.** The stock refuel is the `veh_tank` trigger (item 1), and no non-script stock file raises it. How a station connects to it is ❓. The executable is out of bounds (ADR-0002); the wiki is the only source.
- **Current OPTS refuel.** F12 fills the tank at once, anywhere, when the vehicle is below 0.5 m/s (`fuel-battery.md` §5; the stopped rule was built on 2026-11-23). The stock fill is about 3 units per second, so a 250 tank takes about 83 s.
- **Doc drift.** M-0123's Notes say "the rule 'when stopped' is not applied yet" (2026-11-16 entry), while `fuel-battery.md` §5 and the 2026-11-23 note say it is built. Fix the older line when M-0123 is next updated.

### Options

| Option | What it means | Player impact | Save impact |
| :--- | :--- | :--- | :--- |
| A. Keep F12 when stopped | No station logic. | No stranding; instant refill; any stop works. | None. |
| B. Stations only | Refuel only within reach of a flagged object. | More realistic routing, but a route with no station strands the player unless there is a fallback. | None. |
| C. A plus a station option | F12 stays; a setting adds a timed fill at the stock rate at flagged stations. | Choice for the player; most work. | None. |

### Recommendation: A for M-0123, stations as a follow-up

- A is the milestone's minimum: a vehicle at empty is not stuck. It needs no new data.
- The effect of the keyword in the game is only in the wiki. Building B on it would rest on an unverified fact and risk stranding.
- Follow-up: have the `.sco` parser keep the flag (today it is dropped), find the flagged objects, and offer the timed fill at the stock rate as a station-only option (C).

---

## Impact on M-0123 and the docs

- **Acceptance criterion 3:** change "level × 100" to the stock fraction (item 1).
- **Acceptance criterion 5:** name the source of the level (item 3).
- **`fuel-battery.md` §5:** the "stopped" rule status and the feed scale (items 1 and 4).
- **ISS-042:** close with the two variable families (item 2).
- **Not verified here:** the unit of `engine_tank_content`; who raises `veh_tank`; what the electric engine branches at the 0.01 threshold do; the behaviour of `HC_EBus2019` and `HH20_EBus2019`; the effect of `[petrolstation]` in the game; any add-on family beyond the counts above.
