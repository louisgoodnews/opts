# Script Integration Report (the rig's first corpus run)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0096](../roadmap/phase-4/M-0096.md) (phase P0 of the [bus script integration](bus-script-integration.md)) |
| **Status** | 🟢 M-0096 complete 2026-10-18: rig, stubs, tables, the game's feed from the table, the contract fixes and the NG272 golden run |
| **Reproduce** | `cargo build --release -p opts-bevy --example script_rig` then `target/release/examples/script_rig "<OMSI root>" --json report.jsonl` |

`script_rig` loads the scripts of every `.bus` of an installation, runs them headless for 720 frames (12 s) at 60 Hz under a scripted profile (the driver starts the vehicle as the game does: battery switch, engine start key once the busbar has voltage, *Drive*, parking brake released; then 70 % throttle from the first to the sixth second and a 50 % service brake from the eighth to the tenth; the speed follows from a crude longitudinal model, not the real physics), with the real host callbacks (`opts_bevy::host_callbacks`) and the feed of the [input table](../../../crates/opts-sim/src/script/inputs.rs). `--vehicle <text>` selects vehicles by path and prints their report; with `--csv DIR` it writes every non-zero float variable per frame. One JSON line per vehicle: the unfed inputs, the unconsumed outputs, the callbacks called, answered by the host, stubbed or unknown, the load and run-time diagnostics, the stack underflows, non-finite variables and the cost per frame.

## First corpus run (1,223 vehicles with scripts, the local installation, 2026-10-18)

| Measure | Result |
| :--- | :--- |
| Vehicles whose scripts failed to compile or load | **0** |
| Vehicles with a run-time **stack underflow** | **0** |
| Vehicles with a **non-finite** (NaN, infinity) variable in 720 frames | **0** |
| Cost of one `{frame}` (with the start-up triggers) | median 84 µs, p90 109 µs, max 142 µs |
| Callbacks called by at least one vehicle | 39 of the 45 of OMSI's lists; 34 answered by the host in the run |
| Callbacks answered by a stub / unknown to the registry | 0 / 0 (the host implements every callback the run reached) |
| Unfed inputs | 20 names (below) |
| Outputs nothing reads yet | 16 names (below) |

**Unfed inputs** (names a script reads, no script writes, the input table does not feed; vehicles in brackets): `snd_microphone` (444), `ai_blinker_l/_r` (153 each), `ai_light` (56), `ai_brakelight` (42), `cabinair_relhum` (15), `ai_interiorlight` (10), `axle_springfactor_3_l/_r` (6 each), `cabinair_temp` (4), `axle_brakeforce_2_l/_r` and `_3_l/_r` (2 each), and one vehicle each for `bremse_abs_eingriff`, `bremse_feststell`, `bremse_halte`, `bremse_halte_sw`, `bremse_p_brzyl_ha`, `elec_busbar_main`. The `ai_*` ones are set by the traffic system for AI vehicles (`AiVariables`), not by the player feed; the others are declared by other parts or by outside systems (`snd_microphone`).

**Outputs nothing reads yet** (the scripts write them; vehicles in brackets): `ai_scheduled_atstation` (1,145), `target_index_int` (1,059), `axle_springfactor_*` (1,022), `refresh_strings` (1,016), `snd_outsidevol` (1,015), `tank_percent` (1,004), `cabinair_temp` (986), `ff_vib_amp` and `_period` (984), `cabinair_abshum` (978), `giventicket` (938), `snd_radio` (801), `debug_*` (776), `snd_microphone` (529), `throttle` (202), `brake` (22). They are the work list of the later phases: `target_index_int`, `refresh_strings` and `giventicket` are the route, display and ticket layers (P3 to P5), the sound and force-feedback names are P6, `axle_springfactor_*` is the suspension of the physics contract (P1).

**Load warnings** are plentiful (20,026 over the 1,223 vehicles; their kinds are not analysed here) and none is an error; the run-time messages that are not callback stubs are mostly constants and macros a vehicle reads or calls without defining them (`matrix_deslocamento_max_rear` in 20 vehicles, `bus_length` in 12, single undefined macros such as `ibis_init`). The [content issue registry](../quality/content-issue-registry.md) is their home.

## What changed in the engine (M-0096)

- **Callback stubs.** `callbacks.rs` (the 45 signatures of OMSI's callback lists with the neutral answer of each) and `inputs.rs` (the input and output tables, the feed plan) were committed on 2026-10-17 but never declared as modules, so they were neither compiled nor tested. They are now `opts_sim::script::callbacks` and `opts_sim::script::inputs`. A callback the host does not answer (`ExternalHandler` returns `false`, or there is no handler) is answered by its signature's stub: the arguments leave the stacks, neutral results arrive (`CallbackSignature::apply_stub`), it is counted (`ScriptEngine::callback_stats`) and reported once per engine. Before, such a callback left its arguments on the stack and misaligned every later operation of the block. Tests: `apply_stub` for all 45 signatures on both stacks, and a script per signature that probes the float stack's depth after the call.
- **Usage by name.** `ScriptEngine::usage()` lists the variables, system values and callbacks the compiled scripts read, write and call: the rig compares it with the tables.

## The contract fixes (done 2026-10-18)

A script-written pedal keeps its value (the feed leaves names the scripts store to alone); the start-up system runs before the script system; scripts run for the player's vehicle and those within 150 m (`ScriptRadius`); `GetTime` keeps its resolution (f64, wrapped at 65,536 s); the random seed of a vehicle comes from its `.bus` path and session slot; every compile problem is logged once with file and line; the repaint `[setvar]` values and the player's equipment options are set before `{init}` (M-0078). Tests: `opts-bevy/tests/script_contract.rs`, `seed_tests` in `omsi_world/vehicle.rs`, `opts-sim/tests/callback_stubs.rs`.

## Golden run of the MAN NG272

`script_rig "<OMSI root>" --vehicle MAN_NL_NG/MAN_GN92_main.bus --check crates/opts-bevy/tests/golden/script-rig-gn92.json` runs the vehicle through the profile and compares `elec_busbar_main`, `engine_on`, `engine_n`, `m_wheel`, the gear variables, the brake circuit pressure, the brake forces and `n_wheel` at 3, 6, 9 and 11 s with the golden (2 % or 0.001). At 6 s under 70 % throttle the engine runs at 1,616 rpm and hands 1,411 N·m to the wheels; at 9 s, in the brake phase, the torque is negative (the engine brake). It needs the installation, so it is a local gate (and the numbers are the scripts' outputs under this profile, not OMSI's: the unit of `M_Wheel` is still an open experiment).

## The physics contract (M-0097, 2026-10-18)

Under script drive (now the default for every vehicle whose scripts write `M_Wheel`; `--script-drive` is gone) the torque and the brake forces of the scripts act on **spinning wheels** (`opts-sim/src/vehicle/wheels.rs`): each road wheel has an inertia (`m r²`, 90 kg ❓), receives its share of `M_Wheel` (equal parts over the driven wheels, all wheels when no axle is marked driven) and of the brake force (by rated axle force), and the tyre turns the rim-to-road slip into a longitudinal force, linear up to a 12 % slip and then held at `μ · load` (`μ` = 0.85). A brake torque that would reverse a wheel locks it; a locked or spinning wheel sits at the friction limit, so **no script output can produce a force above it**. The wheel speeds feed `Wheel_RotationSpeed_*` and `n_Wheel` back to the scripts. Standing (below 1 cm/s) the static hold of the earlier model applies with the wheels at rest. The following units of a consist keep their brake share, limited by `μ ·` their axle loads.

`Axle_Springfactor_<n>_<l|r>` scales the spring of that wheel (held to 0.2..5); the zero-speed guard of the chassis counts the scripts' brake, so a held vehicle does not roll through zero.

**Fallback (acceptance criterion 5, tested in `opts-bevy/tests/drive_fallback.rs`):** non-zero torque from the scripts is used; scripts that hand no torque to the wheels for 4 s under throttle with the engine on fall back to the built-in drivetrain with the const file's torque curve (one `INFO` per vehicle); scripts that write no `M_Wheel` at all (or do not compile) get the built-in constants (one `WARN`).

**Closed-loop numbers** (`script_rig --physics`, MAN SD77, 10.9 t, `M_WHEEL_UNIT` = `BRAKE_FORCE_UNIT` = 1.0): 0 to 50 km/h in 16.8 s, stop from 50 km/h in 25.9 m, parking brake holds on 10 % (5 mm), throttle sweep monotone; **the same at 30, 60 and 120 Hz within 1 %** (16.80/16.82/16.83 s, 25.9/25.9/25.9 m). NG272: stop from 50 km/h in about 15 m, held on 10 % with 0.15 m drift, does not reach 50 km/h in the rig's window (36 km/h after 40 s). `--physics --golden FILE` / `--check FILE` write and compare (5 %) the report; goldens: `crates/opts-bevy/tests/golden/physics-{gn92,sd77,ai-kipkw1}.json`.

**Not met, and why:** the acceptance ranges for SD200 (8-14 s, 12-20 m) are not met with unit 1.0. Whether `M_Wheel` is N·m at the wheels and the brake forces are N is exactly the open experiment **E1**: in OMSI, put a vehicle of known mass on a flat road at full throttle in first gear and read the acceleration (and the brake distance from 50 km/h at a known pedal value), then set `M_WHEEL_UNIT` and `BRAKE_FORCE_UNIT` (`opts-sim/src/vehicle/mod.rs`) to the ratio. Until then the constants are 1.0 and the ranges are open; the goldens pin the current behaviour, not OMSI's. The AI-car golden is a closed loop without the traffic system's driver: with the engine off and nothing braking it rolls away on the grade (as a free vehicle would). The `HH109` bus named in the criterion was not located among the sampled vehicles (the sampling cap forbids a sweep), so the third golden is the AI car of the HH109 set.

## Not done (M-0096 follow-ups)

The items below belong to later phases (M-0097 to M-0103): the wheel-level physics contract, per-part script engines and `[scriptshare]` stores for consists, the data layers for routes and tickets, script textures, passengers, sound, AI scripts.

The contract fixes (a script-written pedal keeps its value; `script_startup_system` ordered before `vehicle_script_system`; scripts only for the player's consist and vehicles in the scripting radius; `GetTime` precision; a seed per vehicle from a stable id; every compile problem logged once with file and line), the repaint `[setvar]` values before `{init}` for every vehicle, the game's own feed generated from the input table (the rig uses it, `opts-bevy`'s vehicle script system still has its own), golden runs for the MAN NG272, and the updated `script-vm.md` / `predefined-variables.md`.
