# Bus Script Integration: Strategy for Running the OMSI Scripts of the Player Buses

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Strategy proposal (2026-10-14), not implemented |
| **Evidence** | [Research report](../meta/bus-script-integration-2026-10-14.md), [callbacks reference](../formats/text/vehicles/callbacks.md) |
| **Refines** | [Vehicle scripts route](vehicle-scripts-route.md) (phases VS0-VS7) |
| **Milestones** | [M-0096](../roadmap/phase-4/M-0096.md) to [M-0103](../roadmap/phase-4/M-0103.md) |
| **Builds on** | [M-0026](../roadmap/phase-4/M-0026.md) (VM), [M-0045](../roadmap/phase-4/M-0045.md) (bridge), [M-0053](../roadmap/phase-4/M-0053.md) (articulation and consist mass), [M-0027](../roadmap/phase-4/M-0027.md) (audio), [M-0095](../roadmap/phase-4/M-0095.md) (AI profiles) |

Evidence tags as in the report: **[K]** code, **[C]** corpus, **[R]** run, ❓ inferred or to be settled by experiment.

---

## 1. Goal

**Every script output of a player bus has a consumer, every script input a source, and the script decides what the vehicle does.** Concretely, for a bus loaded from its own files:

1. it starts up, drives, brakes and holds on a grade as its scripts say, on a physics world that only enforces friction and geometry;
2. cockpit, lights, doors, indicators, wipers, climate respond through the scripts' variables;
3. destination boards, IBIS and matrix displays show the right text because the scripts' own drawing calls are executed;
4. passengers board and alight through doors the scripts open, tickets are sold and change is given through the scripts' cash logic;
5. announcements and the vehicle's sound set follow the scripts' events;
6. a coupled consist (articulated bus, tram) is one logical vehicle with one state, whatever the number of parts;
7. nearby AI vehicles run their small scripts (the same contract with the pedals written by the traffic planner);
8. everything is deterministic (fixed step, seeded randomness) and fits a frame budget.

**Non-goals:** a script editor or generator (see [script generator](script-generator.md), M-0092/0055/0056), scheduled AI bus traffic (M-0049 phase 2), replacing OMSI's own content, any change to the script language.

## 2. What the evidence says

Numbers from the [research report](../meta/bus-script-integration-2026-10-14.md) (1,089 player buses unless stated).

| # | Finding | Consequence for the strategy |
| :--- | :--- | :--- |
| F1 | **Demand is all-or-nothing.** Strict coverage is 19 buses (1.7 %) at T1 and 21 at T2; 97.5 % need sound and events (T6). The median bus needs 35 input families and about 30k reachable instructions | There is no cheap "first bus" by capability; progress must be measured per *need*, not per bus, and the final state is only reached when all layers exist |
| F2 | **The first cliff is route and timetable data**, not physics: 1,054 buses (96.8 %) call `GetTerminusString`, 1,057 write `target_index_int` | The data layer (hof, timetable, line selection) is as important as torque and must be planned early |
| F3 | **An unimplemented callback corrupts the script**: its arguments stay on the stack and every later operation is misaligned [K]. 34 of 39 signatures are now inferred with high confidence | A **signature-correct stub layer** is the cheapest, most valuable first step: every bus then runs correctly with blank displays (100 % at the T4 "variables only" level), instead of undefined behaviour |
| F4 | **28 % are articulated, 40 are rail.** Coupled parts get no engine today [K]; control cables and `[scriptshare]` are unhandled | One script *state per consist* is a structural requirement; it must exist before doors, passengers and rail can be right |
| F5 | **Hidden host demands**: 872 buses read `.cti` `[setvar]` livery names, 602 read `articulation_*`; 108k (bus, trigger) pairs have no binding, many probably engine-raised | The input feed needs a *declared table* with provenance, and unfed names must be logged per vehicle |
| F6 | The code is **further along than the route says** and has **defects** (section 5 of the report): backward roll under script brakes, host-overwritten pedals, unseeded RNG, scripts running for parked vehicles | Fix the contract bugs in the first phase; they distort every later measurement |
| F7 | Semantics that need OMSI: `M_Wheel` unit, `STTextOut` arguments 5 and 6, `STSetColor`, `GetHeightAbovePoint`, passenger index space, engine-raised triggers | Build with **named calibration constants and flags**, ship experiments to the user (section 8), never hard-code a guess |
| F8 | AI scripts: 317 of 426 listed vehicles follow the shared law; most deviating lorries yield zero torque with today's inputs [R] | Input completeness is also the AI blocker; one input table serves both |

## 3. Principles

1. **Scripts decide, physics bounds.** The built-in models stay as fallback (script missing, failing, far away); a script cannot produce more force than friction carries.
2. **Honest degradation.** A missing service never silently returns nonsense: a stack-correct stub logs one `ERROR` with vehicle, script and callback, and the integration report counts it. No placeholder text on displays.
3. **Measure integration, not features.** A headless rig runs every bus and emits a machine-readable **integration report** (unfed inputs, unconsumed outputs, callbacks stubbed vs real, stack underflows, diagnostics). Every phase is accepted by a metric on that report, not by a demo.
4. **One input table.** All engine-supplied variables come from one declared table (name, unit, source system, update rate); the feed is generated from it; reading an unfed name is a diagnostic.
5. **Deterministic.** Fixed physics step (the step size is a recorded parameter), a seed per vehicle derived from a stable vehicle id, all time through the simulation clock.
6. **Clean room.** Only shipped text files, wiki facts and measurements; behaviours are validated by plausibility tests and experiments, never by inspecting OMSI's code ([ADR-0002](../adr/0002-clean-room-policy.md)).
7. **Bounded cost.** Per-consist and per-frame budgets are enforced and logged (section 7).

## 4. Architecture decisions

**D1: A script host per consist.** Replace the per-entity `VehicleScripts` by a `VehicleScriptHost` owned by the *consist root*: one `ScriptEngine` per part with its own script set, shared variable stores for `[scriptshare]` parts, the control-cable bridge between parts, the order of execution (front to back), and trigger routing to the part that owns a binding. This matches how OMSI structures state ([route](vehicle-scripts-route.md) §2.1: private copies of shared names per part, state crossing via cables).

**D2: Input table.** `predefined-variables.md` becomes machine-readable (`opts-sim/src/script/inputs.rs`): name, unit, source (`Physics`, `Clock`, `Weather`, `Environment`, `Passengers`, `Timetable`, `Repaint`, `Constant`). `feed_predefined` is generated from it; the rig checks that every name a set reads is fed, declared by the set or an accepted zero.

**D3: Typed callback registry.** `opts-sim` gets a `CallbackRegistry` with a *signature* per name (float arguments, string arguments, float results, string results) from the [callbacks reference](../formats/text/vehicles/callbacks.md). The VM pops and pushes according to the signature even for a stub; a real implementation is a trait object (`RouteService`, `TimetableService`, `TicketService`, `PassengerService`, `CanvasService`, `GroundService`) implemented in `opts-game`/`opts-bevy`. Unregistered names keep today's behaviour plus an `ERROR`.

**D4: Wheel-level physics contract.** `opts-sim` gains a per-wheel angular model (inertia, torque and brake per wheel, slip limited by a friction circle with load transfer kept simple), `M_Wheel` distributed over the driven axles of `[newachse]`, `Axle_Brakeforce_*` per wheel and `Brakeforce` split by load, `Axle_Springfactor_*` into suspension stiffness, rolling resistance from `[rollwiderstand]`, drag, grade from the terrain. Wheel speeds and `n_Wheel` are *outputs of this model*, not of chassis speed. The unit of `M_Wheel` is one constant (`M_WHEEL_UNIT`) settled by calibration (Q-BSI-1).

**D5: Script textures as a service.** A CPU canvas per `[scripttexture]` of the part running the script; the `ST*` calls operate on it; a dirty canvas is uploaded as a Bevy `Image` at a bounded rate; fonts come from a registry keyed by `[newfont]` name. This sits next to the existing `[texttexture]` path, which stays.

**D6: Events as messages.** `(T.L/F/S)` events leave the engine as `ScriptEvent` messages carrying part, kind, name and payload; the audio layer (M-0027) subscribes; nothing is dropped silently.

**D7: Data services behind traits.** The hof (`opts_core::formats::hof`, parser to be written), the timetable and route model, tickets (`.otp`, `Money/*.cfg`), the passenger cabin graph are *data services* in their own modules with their own tests; the callback registry only adapts them to the stacks.

**D8: Scheduling.** Full script sets run for the player's consist and for consists within a configurable radius (default 150 m); parked and far vehicles keep their last state and are advanced by a cheap idle model; AI vehicles use the one-script path (P7).

## 5. Phases

Dependencies: P0 first; then two tracks in parallel (**A** vehicle dynamics: P1 → P2 → P7; **B** information: P3 → P4 → P5); **C** sound (P6) after audio exists.

| Phase | Milestone | Delivers | Acceptance (all measurable without an OMSI oracle) | Rough size ❓ |
| :--- | :--- | :--- | :--- | :--- |
| **P0** Rig, inputs, safety net | [M-0096](../roadmap/phase-4/M-0096.md) | `script_rig` and the integration report; typed callback registry with stack-correct stubs for all 39 callbacks; generated input table and complete feed (date, time, weather, `Clutch`, `StreetCond`, `Envir_Brightness`, `kmcounter_*`, …); fixes for the contract bugs (pedal ownership, startup ordering, player filter, `GetTime` precision, per-vehicle seed, per-vehicle compile diagnostics); repaint `[setvar]` before `{init}` | corpus run: 0 stack underflow at callback sites, 0 NaN/∞ over 600 frames for all 1,089 buses, every unfed name listed; report committed; unit tests for the feed | 3-4 weeks |
| **P1** Physics contract | [M-0097](../roadmap/phase-4/M-0097.md) | D4: wheel model, torque and brake distribution, springfactor, resistances, spring-brake hold, backward-roll fix, `M_WHEEL_UNIT` calibration, script drive becomes the default whenever a set writes `M_Wheel` | SD200 0-50 km/h in 8-14 s, braking from 50 km/h in 12-20 m, holds on a 10 % grade with the parking brake; throttle sweep monotone; same result within 5 % at 30/60/120 Hz | 6-8 weeks |
| **P2** Consist state | [M-0098](../roadmap/phase-4/M-0098.md) | D1: per-part engines, `[scriptshare]` stores, control-cable parse and bridge, trigger routing, `articulation_*` single writer, `[couple_front]`; rail on its scripts' torque | GN92: rear doors open from their own hot-spots; NF6D cable pairs verified in the rig; 23-part consist ≤ 1 ms per step; rail golden run | 5-7 weeks |
| **P3** Route and timetable data | [M-0099](../roadmap/phase-4/M-0099.md) | D7: `.hof` parser, timetable and route model, the 25 route/timetable callbacks, line and destination selection (UI and control protocol), `target_index_int`, `schedule_active`, `Refresh_Strings`, the strings `number`, `yard`, `act_busstop`, `SetLineTo`, `ident` | for SD200, HH109 and MAN EN92: choosing line and course yields the strings the hof defines in the scripts' own string variables; 0 route callback stubs left on those three | 8-10 weeks |
| **P4** Script textures | [M-0100](../roadmap/phase-4/M-0100.md) | D5: `ST*`, `GetFontIndex`, `TextLength`, `[scripttexture]`/`[useScriptTexture]`/`\S:n`, upload budget | destination board of SD200, HH109 and NF6D rendered; layout equals the `TextLength` arithmetic; ≤ 0.5 ms per frame with 8 canvases; golden images | 6-8 weeks |
| **P5** Doors, passengers, tickets | [M-0101](../roadmap/phase-4/M-0101.md) | `PAX_*`, `humans_count`, `GetHumanCountOn*`, cabin graph, `GiveChangeCoin`, ticket callbacks, `GivenTicket`, cash desk | passengers enter only through open doors; HH109 ticket sale gives the right change; counts match the spawned passengers | 8-10 weeks |
| **P6** Sound and force feedback | [M-0102](../roadmap/phase-4/M-0102.md) | D6 plus `sound.cfg` parser, mixer binding, `Snd_*`, `(T.F)` announcements, `FF_*` | every `[trigger]` of the three reference vehicles is reached by an event in a scripted run; ≥ 90 % of `sound.cfg` triggers matched | 6-8 weeks (needs M-0027) |
| **P7** AI scripts near the player | [M-0103](../roadmap/phase-4/M-0103.md) | `{frame}`/`{frame_ai}` for near AI vehicles with planner pedals, LOD switch to the kinematic path, `AiResponse` as the cheap model | 40 near vehicles ≤ 3 ms per step; no speed jump above 0.3 m/s at the switch; scripted and kinematic acceleration within 15 % | 3-4 weeks |

Total on the order of 45-60 engineer-weeks; the figures are rough planning aids, not commitments ❓.

### What a user sees after each phase

P0: nothing new, but every bus loads without undefined behaviour and the report says exactly what is missing. P1: a bus starts, drives and brakes by its own scripts. P2: articulated buses and trams are correct as consists. P3: lines and destinations can be chosen; the strings exist (displays still blank if they need textures). P4: displays light up. P5: passengers and tickets. P6: sound. P7: AI vehicles near the player behave like the player's own vehicle class.

### Why this order

P0 first because it makes every later measurement trustworthy and removes undefined behaviour (F3, F6). P1 before the data layers because physics decides how the vehicle moves, and the data layers are then testable in motion. P2 before P5 because passenger indices and door hot-spots are chain-wide. P3 and P4 are independent of P1/P2 in code and can run in parallel; P4 only needs test scripts until P3 supplies content. P6 waits for the audio engine regardless. P7 reuses P1.

## 6. Verification strategy

* **The rig is the backbone.** `script_rig` (headless, in `opts-sim` examples) loads a vehicle's script set, applies a scripted input profile (pedals, speed ramp, door requests, line selection) and logs all variables over time; a corpus mode produces the integration report. CI runs it where an installation is available (OMSI-gated, like the other corpus tests) and ratchets: the numbers of unfed inputs, stubbed callbacks and diagnostics may only fall.
* **Golden runs** for reference vehicles (SD200, HH109, MAN EN92, GN92, NF6D A/C/B, one AI car): CSV of outputs over a fixed profile, compared with tolerances; updated deliberately with a note.
* **Contract tests per callback:** for every signature the test pushes arguments, calls, and asserts the stack depth and result types, so a wrong arity can never come back.
* **Plausibility tests** for physics (acceleration, braking distance, grade hold, step-size independence) with published orders of magnitude, in the style of the route's VS2/VS3 acceptance.
* **Budgets in the report:** instructions and microseconds per frame per bus, the sum over a consist, canvases uploaded per frame.
* **Experiments** with OMSI itself are listed below and are inputs to constants, never to code structure.

## 7. Budgets

A median bus runs about 30k reachable instructions per frame ([C] 71-100 µs); a 23-part consist about 0.5-1 ms; 40 near AI vehicles about 3 ms; script-texture upload at most 8 canvases per frame and 0.5 ms; script drive adds one wheel-model step per axle. The host reports and logs any overrun; per-vehicle LOD (D8) is the release valve.

## 8. Experiments that need OMSI (hand-over list)

Run by the project lead on a real OMSI; each yields a number or a recorded state that fixes one constant or signature. Calibration of AI behaviour is in [AI calibration protocol](ai-calibration-protocol.md).

| ID | Experiment | Settles |
| :--- | :--- | :--- |
| E1 | 0-50 km/h and standing-start behaviour of three vehicles of known mass (script torque readout) | `M_WHEEL_UNIT`, Q-BSI-1 |
| E2 | `STTextOut` with `a` in 0-3, `b` in {0, 3, 10, 100, -1} on a 256×32 canvas, read back by `STReadPixel` | argument meaning |
| E3 | `STSetColor` with distinct A/R/G/B over fresh and existing pixels | order and blending |
| E4 | `GetHeightAbovePoint` beside a known kerb, kneeling on and off | frame, sign, no-hit value |
| E5 | `STLoadTex` with existing, missing and differently sized files | semantics, scaling |
| E6 | One passenger on a known link or seat of the rear part of an articulated bus, loop indices | passenger index space |
| E7 | Record all events and triggers raised by the engine itself in a run | Q-BSI-4, engine-raised set |
| E8 | A key bound in `keyboard.cfg` for a trigger defined in two parts | trigger routing, Q-BSI-5 |

## 9. Risks

| Risk | Handling |
| :--- | :--- |
| `M_Wheel` unit or distribution wrong makes every bus too strong or weak | one constant, calibrated on three vehicles (E1); friction clamp; plausibility tests |
| Breadth of the data layer (27,729 `.hof`, timetables, tickets, passenger cabin) | separate services with their own parsers and corpus tests; P3 limits itself to what the three reference vehicles and the callbacks need first |
| Display fidelity depends on unverified `ST*` details | E2-E5; defaults chosen so that the common `0 0` case is exact; golden images reveal drift |
| Frame budget with many scripted consists | D8 radius, report-driven budgets, idle model for parked vehicles |
| Per-part state and cables get the order wrong | cable bridge tested against the NF6D's declared pairs; run-order experiment in the rig (Q-VS-1) |
| Reserved-key policy conflicts with scripts that expect OMSI's bindings | report which bindings are dropped; make the filter configurable per vehicle |
| Scope creep into a script editor | non-goal; M-0092/0055/0056 stay separate |

## 10. Alternatives considered

1. **Reimplement each bus natively and ignore scripts.** Rejected: 400 distinct script sets, add-ons ship their own, and the scripts *are* the specification of behaviour.
2. **Keep running scripts only for visuals (today's state).** Rejected as the end state: physics, doors, passengers and sound would stay approximations.
3. **One engine for the whole consist with merged variables.** Rejected: OMSI keeps private copies of shared names per part and exchanges state through cables; a merged store would change script semantics.
4. **Compile scripts to native code or WASM.** Not needed: the VM costs 0.6 % of a 60 Hz frame for a full bus; the risk is breadth, not speed.
5. **Implement tiers T1 to T6 strictly in order.** Rejected: T1-T3 alone serve 2 % of the buses (F1); the safety net (P0) plus physics (P1) is the shortest path to a bus that drives correctly.
6. **Treat displays as optional.** Rejected: 86 % of the buses draw their destination boards with the `ST*` API.

## 11. Open decisions for the project lead

1. Approve the phase order and the two-track plan (or choose a different first target vehicle for P1-P5).
2. Make script drive the default once P1 passes (retire `--script-drive`).
3. Choose the audio crate and mixer design (M-0027) so P6 can start.
4. Schedule the OMSI experiments E1-E8 (about half a day of work).

## 12. Provenance

Derived from the [research report](../meta/bus-script-integration-2026-10-14.md): a code audit of master at `f531a3e`, a corpus scan of 2,752 vehicle definitions and 4,913 scripts, and a signature analysis of 64,792 callback call sites. Estimates are the author's judgement.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 13. Related Documents

[Vehicle scripts route](vehicle-scripts-route.md) · [Script VM](script-vm.md) · [Vehicle-ECS bridge](vehicle-ecs-bridge.md) · [Articulation](articulation.md) · [Rail vehicles](../simulation/rail-vehicles.md) · [`.osc`](../formats/text/vehicles/osc.md) · [Predefined variables](../formats/text/vehicles/predefined-variables.md) · [`.hof`](../formats/text/vehicles/hof.md) · [`sound.cfg`](../formats/text/vehicles/sound_cfg.md) · [AI vehicle configuration](ai-vehicle-config.md)
