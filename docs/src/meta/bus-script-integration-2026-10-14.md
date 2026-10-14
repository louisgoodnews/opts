# Bus Script Integration: Research Report (2026-10-14)

| Metadata | Value |
| :--- | :--- |
| **Branch** | `research/bus-script-integration` (documentation only) |
| **Base** | `master` at `f531a3e` |
| **Question** | What does it take to run the unmodified OMSI scripts of the player buses completely, and where does OPTS stand? |
| **Follows** | [Strategy](../architecture/bus-script-integration.md), [callbacks reference](../formats/text/vehicles/callbacks.md), milestones [M-0096 to M-0103](../roadmap/phase-4/M-0096.md) |
| **Builds on** | [Vehicle scripts route](../architecture/vehicle-scripts-route.md) (2026-10-12), [Script VM](../architecture/script-vm.md) |

Evidence tags: **[K]** read in code (file and line), **[C]** counted over the installed corpus, **[R]** observed in a run, ❓ inferred or unverified. Nothing in this report was verified against OMSI itself.

---

## Summary

1. **The VM is not the problem; the host is.** All 6,146 corpus scripts compile and run. What stops a bus from being *fully* script-driven is everything around the VM: the physics contract (`M_Wheel`, brakes), per-part state for coupled vehicles, 41 of 42 engine callbacks, the script-texture API, passengers and tickets, and sound events.
2. **Demand is all-or-nothing.** Of the **1,089 player buses** (`.bus` with scripts and a passenger cabin), **1.7 % (19 buses)** have every need met by a host with pedals, speed, wheel speeds and time (tier T1), and **1.9 % (21)** with torque and brakes (T2). Almost every bus needs the route and timetable callbacks (96.8 %), tickets (90.2 %), the script-texture API (86.0 %), events and sound outputs (97.5 %). The first real coverage cliff is **route and timetable data**, not physics.
3. **A cheap safety net closes most of the gap on paper.** With stack-correct stubs for the callbacks (their signatures are now inferred: 34 of 39 with high confidence) and events/sound outputs allowed to degrade, **all 1,089 buses run with correct stack discipline** (tier T4 "core" coverage, 100 %); displays stay blank until the hof, timetable and canvas layers exist. Today an unimplemented callback leaves its arguments on the stack and misaligns every operation after it.
4. **28 % of the buses are articulated** (306: `[couple_back]` plus a `[scriptshare]` trailer) and 40 are rail (all NF6D). Coupled parts get **no scripts and no state of their own today**; one state per coupled chain is a requirement, not an option.
5. **The code is further along than the route document says, and has real defects.** Wheel speeds, `n_Wheel`, `Velocity_Ground`, `A_Trans_*`, vehicle `[texttexture]` and `NrSpecRandom` are already there; `M_Wheel` and the brake outputs drive the physics under `--script-drive`; eight statements of the route document's state table are out of date (section 3). The audit also found 15 suspicious spots, among them a rolling-backward hazard in the script-drive path, a host that overwrites script-written pedals, and a random generator that is never seeded per vehicle.
6. **Open semantics need OMSI, not more reading.** Five callback signatures (`STTextOut` arguments 5 and 6, `STSetColor`, `GetHeightAbovePoint`, `STLoadTex`, passenger index space) and the unit of `M_Wheel` can only be settled by experiments on a running OMSI; they are listed with the exact experiment.

---

## 1. Method

Three independent read-only investigations plus one measurement, all over the same installation (2,752 `.bus`/`.ovh` definitions, 4,913 vehicle `.osc` files); nothing was written to the installation, no executable was opened:

* **Code-state audit** of the repository at `f531a3e`: every layer of the integration, with file and line.
* **Corpus demand scan** (Python): parses definitions, scripts, varlists, `model.cfg`, `sound.cfg`, `keyboard.cfg`, `gamectrler.cfg` and `.cti` repaint files, resolves script paths case-insensitively, honours `[scriptshare]`; reports needs *per bus*, not per use.
* **Callback signature inference**: token scan with a conservative stack-depth analysis at all 64,792 call sites, cross-checked with `.hof`, `.oft`, `Money/*.cfg`, `.otp` and `model.cfg` data. Result: the [callbacks reference](../formats/text/vehicles/callbacks.md).
* **AI script response** (follow-up of M-0095): the VM runs the AI script of every vehicle in the map AI lists (section 6).

Population definitions: **player buses** = 1,089 `.bus` files with `[script]` and `[passengercabin]` (1,049 road, 40 rail); wider sets: 1,224 scripted `.bus`, 1,237 scripted `.ovh`. `Addons/` contains no vehicle definitions. The player buses share only 400 distinct script sets in 94 vehicle folders (two Kajosoft Citybus 530 folders alone contribute 390 files); counts below are per `.bus` file.

## 2. Where OPTS stands (code, per layer)

Status: ✅ done, 🟡 partial, ❌ missing. File references are relative to the repository root.

| Layer | Status | Evidence and what is missing |
| :--- | :---: | :--- |
| Language and VM | ✅ | `opts-sim/src/script/*` (compiler `word_op`, `vm.rs`); stores keep their value, `/` and `%` by zero give 0, stacks capped at 512 (oldest values dropped silently), macro depth 64. Unknown tokens warn (not de-duplicated, `compiler.rs:509`); unfed system values read 0 silently (`vm.rs`, `Op::Sys`) |
| Determinism | 🟡 | Fixed step 64 Hz default, 20-1000 Hz configurable (`opts-cli/src/args.rs:80`, `opts-game/src/app.rs:95`); `GetTime` is an `f32` sum (`mod.rs:297`); RNG xorshift32 with one default seed, **`seed_random` is never called** |
| Script set loading | 🟡 | `attach_scripts` for the head `.bus` only (`opts-game/src/in_game/omsi_world/vehicle.rs:1171-1226`); repaint `[setvar]` applied before `{init}`; **`[scriptshare]` parsed but never resolved; coupled parts get no engine** (`spawn_trailer_chain`, `vehicle.rs:499-690`); `[control_cable_*]` and `[couple_front]` not parsed |
| Host inputs | 🟡 | `feed_predefined` (`opts-bevy/src/vehicle_script.rs:62-101`): `velocity` (km/h), `throttle`, `brake`, `ai`=0, `velocity_ground`, `a_trans_x/y/z`, `ai_engine`=0, `n_wheel`, per-wheel `wheel_rotationspeed_*`, `wheel_rotation_*`, `axle_steering_*`, `axle_suspension_*`=0, `timegap`, `gettime`, mouse while a hot-spot is held, articulation angles. **Not fed:** `Time`, `Day`, `Month`, `Year`, weather, `PrecipRate/Type`, `SunAlt`, `coll_*`, `Pause`, `AutoClutch`, `Clutch`, `StreetCond`, `Envir_Brightness`, `kmcounter_*`, `humans_count`, `schedule_active`, `AI_Target_Index`, `TrafficPriority*`, real suspension travel, strings `number`, `yard`, `act_busstop`, `SetLineTo`, `ident` |
| Host outputs | 🟡 | Consumed: `[newanim]`/`[visible]` (`vehicle_anim.rs:302-335`, `maxspeed` applied, `delay` not), lights and `Spot_Select` (`vehicle_lights.rs`), vehicle `[texttexture]` strings (`vehicle_text.rs`), `M_Wheel`/`Brakeforce`/`Axle_Brakeforce_*` under `--script-drive` only (`vehicle_script.rs:135-189`). **Nothing consumes** `Axle_Springfactor_*`, `PAX_*`, `GivenTicket`, `Snd_*`, `FF_*`, `Debug_*`, script-written pedals; the HUD ignores the scripts (`hud/values.rs:15`) |
| Triggers | ✅ | `keyboard.cfg` chords (Shift/Ctrl exact, no Alt; `_off` only for held bindings), mouse hot-spots with left click (`vehicle_triggers.rs:311-389`), all go to the player's head engine. `OPTS_RESERVED_KEYS` (`:27-65`) silently drops OMSI bindings on W A S D E Q N R, Space, Tab, arrows, digits, several F-keys and others. `gamectrler.cfg` bindings are not read |
| Events | ❌ | `(T.L/F/S)` events are drained and logged at debug level (`vehicle_script.rs:127`); no audio crate, `sound.cfg` parsed as a path only (`bus.rs:132`) |
| Callbacks `(M.V.*)` | ❌ | Only `NrSpecRandom` (`host_callbacks.rs:23-33`, seed from `Entity::to_bits`). The other 41 return false, are warned once and **leave the stacks untouched** |
| Script textures | ❌ | `[scripttexture]` parsed to width and height only (`model/ast.rs:362`); no `[useScriptTexture]`, no `ST*` API. Vehicle `[texttexture]`/`[texttexture_enh]` work (`parse.rs:450-480`, `text_texture.rs`, at most 8 redraws per frame) |
| Script drive | 🟡 | `--script-drive`: tractive force `M_Wheel / r` of the first driven axle, brake from `Axle_Brakeforce_*` else `Brakeforce`, both clamped to 0.85 × mass × g (`opts-sim/src/vehicle/mod.rs:189-233`); N·m assumed; no wheel-slip state, no per-axle distribution, no `Axle_Springfactor`, no grade term; start-up emulation `script_startup_system` (`vehicle_script.rs:192-274`: battery, starter, selector D with brake forced to 1, parking brake). NLC reaches about 13 km/h after 20 s per the docs, **not re-measured here**. No test covers it |
| Doors and passengers | 🟡/❌ | Door animations through triggers and `[newanim]` only; no `PAX_*`, no `humans_count`; `paths.cfg` and `passengercabin.cfg` parsed but unused |
| Displays and timetable | ❌ | No `.hof` parser, no timetable or route model, no line/destination input; matrix and IBIS text appear only where scripts write the string themselves |
| Multi-part and rail | ❌ | No per-part engines, no control-cable bridge; articulation angles feed the head engine; rail runs `RailDynamics` instead of the scripts' torque |
| AI vehicles | 🟡 | Kinematic traffic with `AiVariables`; `run_frame(dt, true)` has **no caller**, so `{frame_ai}` never runs |
| Tests and tooling | 🟡 | `script_vm.rs` (27 tests), `vehicle_script_bridge.rs` (4), `articulation.rs` (8); examples `osc_bench`, `osc_corpus`, `script_diag`, `vehicle_scripts`; **no `script_rig` and no golden outputs**; nothing tests `feed_predefined`, the start-up system, hot-spots end to end or text textures |

## 3. Where the route document is out of date

[`vehicle-scripts-route.md`](../architecture/vehicle-scripts-route.md) section 3 (2026-10-12) differs from the code in eight places:

1. It lists `wheel_rotationspeed_*`, `n_wheel`, `velocity_ground` and `a_trans_*` as missing; all are fed (`vehicle_script.rs:70-90`). `ai_engine` is a constant 0.
2. It says `M_Wheel`, `Brakeforce` and `Axle_Brakeforce` are not consumed; under `--script-drive` they are. Only `Axle_Springfactor` is unconsumed.
3. It says script textures are missing and `text_texture` covers only scenery; vehicle `[texttexture]` exists. Only `[scripttexture]` and the `ST*` API are missing.
4. It says all 42 callbacks are no-ops; `NrSpecRandom` works.
5. "Fixed 60 Hz": the default is 64 Hz and configurable (20-1000 Hz), so integrated script quantities depend on `--physics-hz`.
6. "Seeded generator per vehicle": `seed_random` is never called; all vehicles share one seed.
7. Triggers are marked ✅ without the reserved-key filtering.
8. "Head part only" for loading omits that `[scriptshare]` is not handled at all and `[control_cable_*]` is not parsed.

The route's phases VS0-VS7 stay valid as an outline; the strategy refines their order with the demand data below.

## 4. What the buses ask of a host (corpus demand)

### 4.1 Size of the script sets

| Population | Scripts (median / p90 / max) | Blocks | Static instructions | Reachable from `{frame}` |
| :--- | :--- | :--- | :--- | :--- |
| 1,089 player buses | 27 / 30 / 36 | 628 / 648 / 881 | 40.6k / 44.2k / 74.7k | 30.4k / 33.3k / 55.5k |
| 400 distinct sets | 25 / 29 / 36 | 520 / 649 / 881 | 27.7k / 46.3k / 74.7k | 21.9k / 32.0k / 55.5k |
| 1,237 scripted `.ovh` | 1 / 4 / 6 | 4 / 25 / 82 | 168 / 1,056 / 5,675 | 147 / 868 / 5,124 |

A median player bus declares about 1,750 float variables in 25 varlists. 46 player buses are reduced AI-style sets with at most 3 scripts.

### 4.2 Variables the host must supply (read, never written by the set)

Families over axle and door indices, buses out of 1,089:

| Variable | Buses | Variable | Buses |
| :--- | ---: | :--- | ---: |
| `Velocity` | 1,067 | `Wheel_Rotation_*` | 952 |
| `PrecipRate` | 1,051 | `humans_count` | 948 |
| `Wheel_RotationSpeed_*` | 1,049 | `Throttle` | 897 |
| `Brake` | 1,033 | `Axle_Suspension_*` | 890 |
| `AI`, `AI_Target_Index` | 1,032 / 1,027 | `PAX_Exit*_Req` | 862 |
| `Axle_Steering_*` | 1,023 | `wearlifespan` | 805 |
| `AI_Engine`, `n_Wheel` | 1,022 / 1,019 | `PAX_Entry*_Req` | 719 |
| `Velocity_Ground`, `DirtRate` | 1,016 / 1,010 | `Envir_Brightness` | 560 |
| `kmcounter_km/m` | 998 | `schedule_active` | 494 |
| `A_Trans_X/Y/Z` | 992-996 | `Snd_Microphone` | 444 |
| `StreetCond` | 993 | `Clutch` | 359 |

A median bus needs **35 input families** (maximum 39). System values: `Timegap` 1,079, `GetTime` 1,057, `mouse_x/y` 1,045 / 1,007, `Year` 1,025, `Weather_Temperature` and `Day` 1,022, `Month` 1,020, `Time` 1,015, `Weather_AbsHum` 982, `coll_energy` 961, `coll_pos_*` 539-944, `PrecipRate/Type` 486 / 427, `DayOfYear` 188, `AutoClutch` 124, `SunAlt` 30, `Pause` 22.

**Hidden host demands** (variables that look script-internal): 602 buses read `articulation_<n>_alpha/beta` that only the engine writes; **872 read names that only a `.cti` repaint `[setvar]` sets**, so livery variables must be applied before `{init}`; a residue of 984 distinct declared-but-unwritten names (for example `antrieb_motorleistung`, 973 buses) is zero-initialised, whether another writer exists is unknown ❓.

### 4.3 Outputs the host must consume

| Output family | Buses | Output family | Buses |
| :--- | ---: | :--- | ---: |
| `target_index_int` | 1,057 | `AI_Blinker_*` | 1,013 |
| `M_Wheel` | 1,055 | `AI_Light` | 1,005 |
| `PAX_Entry0/Exit0_Open` | 1,053 | `tank_percent` | 1,004 |
| `Spot_Select` | 1,036 | `Cabinair_Temp` | 986 |
| `AI_Interiorlight` | 1,032 | `FF_Vib_Amp/Period` | 984 |
| `AI_Scheduled_AtStation` | 1,025 | `GivenTicket` | 938 |
| `Axle_Springfactor_*` | 1,022 | `Brakeforce` | 831 |
| `Axle_Brakeforce_*` | 1,020 | `Snd_Radio` | 800 |
| `Snd_OutsideVol` | 1,015 | `Debug_*` | 734 |
| `Refresh_Strings` | 1,014 | `Snd_Microphone` | 529 |

Brake mode: both `Brakeforce` and per-wheel 793 (72.8 %), per-wheel only 227 (20.8 %), total only 38, neither 31; 636 buses write 2 axles of per-wheel brake force, 324 write 3. Doors: 507 buses write 8 door indices, 262 write 4, 115 write 6; 36 write none.

### 4.4 Callbacks (details in the [reference](../formats/text/vehicles/callbacks.md))

39 of the 42 callbacks are used; 34 buses use none; **616 (56.6 %) need all six groups**.

| Group | Buses | Main members |
| :--- | ---: | :--- |
| Route and timetable | 1,054 (96.8 %) | `GetTerminusString` 1,054, `GetDepotStringGlobal` 1,041, `GetTerminusCode` 1,035, `GetBusstopString` 1,006, `GetTTBusstopIndex` 949, `GetRouteIndex` 943 |
| Tickets and cash | 982 (90.2 %) | `GiveChangeCoin` 980, `GetTicketValue` 938, `GetTicketName` 923 |
| Script-texture API | 936 (86.0 %) | `STNewTex`/`STLock`/`STUnlock` 936, `STTextOut`/`GetFontIndex`/`STSetColor` about 928, `STDrawRect` 904 |
| Randomness | 904 (83.0 %) | `NrSpecRandom` |
| Geometry | 839 (77.0 %) | `GetHeightAbovePoint` |
| Passenger counts | 785 (72.1 %) | `GetHumanCountOnPathLink` 756, `GetHumanCountOnSeat` 110 |

### 4.5 Script textures and events

* **Script textures:** 936 buses (86 %) in 312 sets and 62 folders; `[scripttexture]` blocks per bus: median 2, p90 7, max 30. The other buses show text through `[texttexture]` only (SD83 is one). Vehicles ask for a median of 26 fonts (max 75); the commonest are the Kajosoft `kjt3d_*` families (416 buses each) and `streetberlin`/`lcd_7-segment` (217).
* **Events:** 2,043 distinct names, median 199 raised per bus; mostly per-door or per-ABS-wheel variants. Commonest: `ev_lights_blinker_on` 1,015, `ev_bremse_feststell_ein/aus` 1,008, `ev_fahrertuer_auf/zu` 1,007, `wiper_start` 998, compressor and horn events 980-997. **90.3 % of the 41,309 `sound.cfg` `[trigger]` entries name an event that the same set raises**; the unmatched 4,002 (`ev_ticketsale_*`, `ev_stamper`, `ev_stoermeldung`, `ev_matrix_line_change`) are probably engine-raised ❓. `(T.F)` events carry a file name on the string stack (`ev_ibis_ansagen` 485 buses): announcements.

### 4.6 Triggers

5,520 distinct names (median 447 per bus). `keyboard.cfg` binds 129 of them (91 in `[vehicles]`); `gamectrler.cfg` binds 25 more that appear nowhere in `keyboard.cfg` (for example `blinker_left_move`); there are no per-vehicle key files. By (bus, trigger) pair: mouse only 212k pairs (1,002 buses have at least one), key plus mouse 47k, key only 37k, script-raised only 253, **no binding found 108k** (likely engine-raised: `ai_scheduled_settarget` 1,039 buses, `veh_tank` 1,006, `collision` 983, `malfunction_reset` 974 ❓). Only 33 buses have every trigger bound somewhere: mouse operation of the cockpit is not optional.

### 4.7 Multi-part, doors, passengers

| Population | Files | `[couple_back]` | `[scriptshare]` | `[control_cable_*]` | Rail |
| :--- | ---: | ---: | ---: | ---: | ---: |
| Player buses | 1,089 | 346 | 6 | 36 | 40 |
| All `.bus` | 1,492 | 364 | 277 | 38 | 52 |
| Scripted `.ovh` | 1,237 | 668 | 4 | 535 | 834 |

**306 player buses (28.1 %) are articulated** (`[couple_back]` with a `[scriptshare]` trailer, 138 sets in 62 folders); the 40 rail buses are all NF6D parts (36 with control cables). The door, passenger and ticket stack is used by 896 buses: `PAX_*_Open` written by 1,053, door scripts in 1,038, cash desk or ticket printer scripts 948, `humans_count` 948, `GivenTicket` 938, `GetHumanCountOn*` 785.

### 4.8 What a host must offer, in tiers

Tier assumptions: **T0** VM only; **T1** pedals, speed, wheel speeds, time, environment, `NrSpecRandom`; **T2** physics coupling (`M_Wheel`, brakes, `Axle_*`, `articulation_*`, `GetHeightAbovePoint`); **T3** doors, passengers, tickets; **T4** route and timetable strings plus `target_index_int`, `schedule_active`, `Refresh_Strings`; **T5** script-texture API; **T6** sound events, `Snd_*`, `FF_*`, `Spot_Select`. A bus is *covered* at tier k when every need it has lies in tiers up to k.

| Tier | Strict | Hard needs only (outputs and events degrade) | Variables only (callbacks and events degrade) | Median share of needs met | Top unmet needs |
| :--- | ---: | ---: | ---: | ---: | :--- |
| T0 | 0 | 0 | 0 | 2 % | `Velocity` 1,067; events 1,061 |
| T1 | 19 (1.7 %) | 27 | 25 | 37 % | events 1,061; `AI_Target_Index` 1,057 |
| T2 | 21 (1.9 %) | 32 | 28 | 53 % | events 1,061; `target_index_int` 1,057 |
| T3 | 21 | 32 | 28 | 63 % | same |
| T4 | 27 (2.5 %) | 40 | **53 (4.9 %)** | 80 % | events; `Spot_Select` 1,036; `Snd_OutsideVol` 1,015 |
| T5 | 27 | 116 (10.7 %) | 53 | 93 % | events; `Spot_Select` |
| T6 | 1,089 (100 %) | 1,089 | 1,089 | 100 % | none |

If T5 and T6 needs may degrade, coverage is 0 at T0, 25 (2.3 %) at T1, 28 at T2 and T3, and **100 % at T4**. 1,062 buses (97.5 %) need T6; only 19 stop at T1, 2 at T2, 6 at T4. T1-T3 buy almost nothing alone because the standard script library asks for T4 data everywhere. The **gatekeepers**: T4 `AI_Target_Index`/`target_index_int` (1,057), `GetTerminusString` (1,054), `GetDepotStringGlobal` (1,041); T5 the three canvas calls (936); T6 events (1,061) then `Spot_Select` (1,036). Placements of `Snd_Microphone`/`Snd_Radio` (T6), `AI_Target_Index` (T4) and `coll_*` (T2) are my assumptions ❓. The earlier route document counted 338 resolved `.bus` and 280 `M_Wheel` writers; this scan finds 1,224 scripted `.bus` and 1,055 `M_Wheel` writers among the player buses, so its scan was probably partial.

### 4.9 Seven representative buses

| Need | MAN EN92 | MAN GN92 (articulated) | HH109 Stadtbus HHA | MAN SD83 | HH109 Gelenkbus 2012 | Kajosoft 530 Facelift | Tram NF6D |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Scripts / blocks | 26 / 372 | 26 / 372 | 24 / 380 | 24 / 299 | 25 / 432 | 30 / 643 | 6 / 122 |
| Static / reachable instr. | 14.2k / 10.4k | 14.7k / 10.9k | 20.6k / 16.3k | 11.8k / 8.5k | 25.9k / 21.6k | 43.5k / 32.7k | 9.0k / 7.4k |
| Input families (bidirectional) | 25 (10) | 25 (11) | 24 (9) | 25 (9) | 24 (13) | 26 (9) | 12 (3) |
| Brake mode | both | both | per wheel | both | per wheel | both | total |
| Doors written | 10 | 10 | 4 | 6 | 6 | 16 | 8 |
| Distinct callbacks | 29 | 29 | 19 | 12 | 19 | 31 | 26 |
| `[scripttexture]` / `[texttexture]` | 2 / 12 | 2 / 12 | 0 / 33 | 0 / 10 | 0 / 69 | 2 / 39 | 2 / 6 |
| Events raised | 125 | 131 | 130 | 101 | 168 | 244 | 37 |
| `sound.cfg` triggers matched | 91 / 95 | 86 / 90 | 146 / 150 | 71 / 78 | 91 / 94 | 222 / 228 | 34 / 34 |
| Triggers key / mouse only / other | 87 / 111 / 43 | 87 / 113 / 41 | 67 / 175 / 21 | 88 / 64 / 32 | 59 / 207 / 43 | 81 / 303 / 88 | 45 / 37 / 6 |

## 5. Suspicious code found (not fixed)

1. `opts-sim/src/vehicle/mod.rs:202-233`, `chassis.rs:130-137`: the zero-speed guard and the zero clamp use the *driver's* `is_braking()`, not the script's brake. With script brakes holding and a small idle `M_Wheel` the net force at standstill is negative: the bus can **roll backward and jitter** as the brake direction flips; any driver brake press snaps speeds below 0.1 m/s to 0 whatever the script brake does.
2. `vehicle_script.rs:286-288`: `script_startup_system` and `vehicle_script_system` lock the same engine with no ordering.
3. `vehicle_script.rs:64-75`: the host **overwrites `throttle`, `brake`, `ai` and `ai_engine`** every frame, so scripts that write `Throttle` (65 `.bus`, 29 `.ovh` per the docs) lose it.
4. `vehicle_script.rs:106`: no player filter, so every `VehicleScripts` entity (parked session vehicles) runs every step with idle inputs.
5. `script/mod.rs:297`: `GetTime` as an `f32` accumulator loses precision over long sessions.
6. `seed_random` is never called: all vehicles get the same "random" start state (for example the random initial tank pressure the docs describe).
7. `vm.rs`: unfed system values read 0 silently; an unimplemented callback misaligns the stacks; stack overflow drops the oldest values without a diagnostic.
8. `host_callbacks.rs:30`: the seed is `Entity::to_bits`, unstable across runs and save-load; `n.round() as i64 as u64 as u32` wraps for negative `n`.
9. `chassis.rs:116-119`, `vehicle_script.rs:72`: `A_Trans_X` is the lateral-damping term, not an acceleration; `A_Trans_Z` is fixed at 0.
10. `vehicle_script.rs:97`: the `axle_steering` sign is assumed (the code comment says unverified).
11. `rail.rs:537-538` and `ground_height.rs:377-378`: two writers of `articulation_*_beta/gamma`; both running on one entity is unconfirmed.
12. `vehicle_triggers.rs:27-65`: reserved-key filter drops OMSI bindings with a debug log only.
13. `vehicle_anim.rs:316`, `vehicle_lights.rs`: one engine lock and name lookups per animated mesh per frame in `Update`; costly with many meshes.
14. `vehicle.rs:1186-1200`: compile warnings and errors of vehicle scripts are not printed individually (only "file not found" and counts).
15. Rail runs scripts with road-model wheel inputs ❓.

## 6. AI scripts: measured response (follow-up of M-0095)

The AI planner writes `Throttle` and `Brake`; the vehicle's script turns them into `M_Wheel` and `Brakeforce`. A new measurement (`opts_sim::script::measure_ai_response`, branch `agent/ai-characterise`) runs a vehicle's script with the AI flags at eight speeds and at full brake. Over the **426** vehicles of the map AI lists:

| Result | Vehicles |
| :--- | ---: |
| Follow the shared car law `M_Wheel = Throttle · AI_M_gas_max · (1 − v / AI_v_max_erreich)`, `Brakeforce = Brake · AI_F_brems_max` (within 5 %) | 317 |
| Deviate or lack the constants (ships, trains, bicycles, scooters, many lorries) | 85 |
| Give neither torque nor brake force | 23 |
| Without scripts | 1 |

Most deviating lorries yield **zero torque** with the inputs available today: their scripts need more engine inputs (the gearbox and engine chain) than the AI measurement feeds, which is the same input-completeness gap as for buses (section 2). The measurement is useful already (it reproduces the law for 317 vehicles and gives bicycles and scooters real torque curves); for the rest it is a probe of how complete the input feed is.

## 7. Open questions that need OMSI

| ID | Question | Experiment |
| :--- | :--- | :--- |
| Q-BSI-1 | Unit and sign of `M_Wheel` (N·m total, per axle, kN·m?) and how the engine distributes it | Calibrate with three vehicles of known mass: 0-50 km/h times, standing start torque; compare `210000 / gear ratio` expressions of the NF6D |
| Q-BSI-2 | Order of engine writes and `{frame}`: which inputs are the previous frame's | Scripts reading before writing: trace in the rig |
| Q-BSI-3 | `STTextOut` arguments `a`, `b`; `STSetColor` order and alpha; `STLoadTex` semantics; `GetHeightAbovePoint`; passenger index space | The five experiments of the [callbacks reference](../formats/text/vehicles/callbacks.md) |
| Q-BSI-4 | Which triggers does the engine raise itself (`ai_scheduled_settarget`, `veh_tank`, `collision`, `malfunction_reset`, `ev_ticketsale_*`) | Record raised events in a run; match against the 108k unbound pairs |
| Q-BSI-5 | Does a trigger defined in several parts reach all of them | NF6D C-car door with a key bound in `keyboard.cfg` |
| Q-BSI-6 | What `[type]` 2 on a road scooter means (see the [AI investigation](ai-vehicle-config-2026-10-14.md)) | Inspect in OMSI |

## 8. Provenance

* **Read:** the route and VM documents, [`.osc`](../formats/text/vehicles/osc.md), [predefined variables](../formats/text/vehicles/predefined-variables.md), M-0026, M-0045, M-0053; the code listed in section 2.
* **Run (read-only):** the three scans over `<OMSI>/Vehicles`, `Inputs`, `Fonts`, `Money`, `TicketPacks` and `program/*.txt` (text files only; no executables or DLLs); `ai_response` over the AI lists of 45 maps; no game was started for this report.
* **Not done:** any measurement of OMSI itself; any change to the repository other than these documents.
* **Clean-room (ADR-0002):** no OMSI binaries inspected; no GPL code read; no OMSI assets committed. Short token sequences quoted from scripts are facts about file content, not assets.
