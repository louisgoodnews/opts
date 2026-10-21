# OSC Runtime Design

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft — awaiting review |
| **Phase** | Phase 4 |
| **Related** | [M-0096](../roadmap/phase-4/M-0096.md) to [M-0103](../roadmap/phase-4/M-0103.md) |
| **Author** | agent session `docs/osc-runtime-design` |
| **Date** | 2026-10-16 |
| **Builds on** | [Bus script integration strategy](bus-script-integration.md) (decisions D1-D8, phases P0-P7), [research report](../meta/bus-script-integration-2026-10-14.md) |
| **Inputs** | [OMSI scripting guide](../formats/text/vehicles/osc-guide.md), [Vehicle functions](../simulation/vehicle-functions.md), [Script VM](script-vm.md), [Callbacks](../formats/text/vehicles/callbacks.md) |

Evidence tags as in the companions: **[K]** code, **[C]** corpus, **[S]** strategy or spec, ❓ open. Nothing here was verified against OMSI (ADR-0002).

---

## 📖 Overview

This sheet specifies **how OPTS runs the scripts of a vehicle** so that the functions in the [catalogue](../simulation/vehicle-functions.md) behave as their scripts intend: the runtime's layers, the host interface, the variable system, the execution model, the bindings to physics, audio, displays, route and ticket data, and the error, debugging and performance rules. It is written so that an engineer can implement it without further design; it contains no code and no task list.

Its relationship to the other documents:

* The **[strategy](bus-script-integration.md)** decided *what* to build and in which order (D1 script host per consist, D2 input table, D3 typed callback registry, D4 wheel-level physics contract, D5 script textures as a service, D6 events as messages, D7 data services behind traits, D8 scheduling). This sheet designs the runtime those decisions describe and does not reopen them.
* The **[scripting guide](../formats/text/vehicles/osc-guide.md)** says what the language means; every rule in the execution model below is one the guide documents from the corpus or the VM.
* The **[function catalogue](../simulation/vehicle-functions.md)** says what the host must provide; its coverage table is the priority list of this sheet.

**Scope.** First-order demands (strategy §2, catalogue coverage table): route and timetable data, ticketing, script textures, sound triggering, and the *interface* of the physics contract. Named and deferred (the last section): AI scripts near the player, the audio engine itself, rigid-body train physics, passenger simulation.

## 🎯 Problem Statement

What is broken today, with the evidence:

1. **Half of the host is missing.** Of the 39 callbacks, 27 are implemented stack-correctly [K: `opts-bevy/src/host_callbacks.rs`, checked against `callbacks.md` 2026-10-16]; the 12 script-texture callbacks are not, and an unimplemented callback leaves its arguments on the stacks and misaligns the rest of the run [K: `vm.rs` `call_external`; S: research report summary 3]. 936 buses (86 %) draw their destination boards through them [S].
2. **The data behind the implemented callbacks is static.** The host context is filled once at spawn (stop index 0, delay 0) [K: `opts-game/.../vehicle.rs` `VehicleHostContext`]; `act_busstop` and `Refresh_Strings`, which the scripts write for the engine, are never read back [S: research report §2]. So the IBIS can select a route but the bus never moves along it.
3. **Outputs with no consumer.** Events are logged at debug level and dropped; `(T.F.*)` payloads (announcement file names) are lost because the event record has no payload [K: `vm.rs` `raise`, `ScriptEvent`]; `Snd_*`, `FF_*`, `Axle_Springfactor_*`, `PAX_*_Open`, `GivenTicket` are unconsumed [S: research report §2].
4. **One engine per entity, not per consist.** Coupled parts get no scripts and no state; `[scriptshare]` is not resolved; control cables are not parsed [S: research report §2, F4]. 28 % of player buses are articulated and the tram's parts run different scripts joined by 102 cables [C: `NF6D_A.bus`].
5. **The physics contract is incomplete and has defects**: wheel speeds derived from chassis speed (so ABS never engages), script-written pedals overwritten, backward roll under script brakes, no `Axle_Springfactor` consumer [S: research report §5].
6. **Semantics the VM gets wrong against the corpus**, found by this session [C: guide, pitfalls]: `$msg` pops its string although 78 of 325 sites use it afterwards; a script-raised event never runs the script's own trigger although `osc.md` says it does (253 pairs depend on one of the two behaviours ❓); a trigger's return value (`malfunction_gettime`) is discarded; `GetFontIndex` is called every frame and must be cheap.

The VM itself is fast and complete enough (0.6-0.8 % of a 60 Hz frame per bus [S: `script-vm.md`]); the problem is the host around it.

## 🏛️ Architecture

Four layers, three crates. Arrows are data flow; the only calls *into* the VM are the lifecycle calls of its host.

```
┌──────────────────────────────────────────────────────────────────────────────────────┐
│ opts-game / opts-bevy  (ECS, rendering, input, session)                              │
│   ScriptHostSystem ─── per consist, FixedUpdate ─────────────────────────────────┐  │
│     │ inputs (InputTable feed)          ▲ outputs (OutputTable read)             │  │
│     ▼                                   │                                        │  │
│   Subsystem bindings: Physics · Canvas/Display · Audio(events) · Lights/Anim ·   │  │
│                       Triggers (keys, hot-spots, engine) · Session save          │  │
└─────┼───────────────────────────────────┼────────────────────────────────────────┼──┘
      │                                   │                                        │
┌─────▼───────────────────────────────────┴────────────────────────────────────────▼──┐
│ opts-sim::script  (pure Rust, no Bevy)                                               │
│   ConsistScriptHost ── parts[] ── ScriptEngine (VM) ── CallbackRegistry ── services │
│                        cables, scriptshare, trigger routing, event queue, budget    │
└─────┬────────────────────────────────────────────────────────────────────────────────┘
      │ parsed data
┌─────▼────────────────────────────────────────────────────────────────────────────────┐
│ opts-core  (formats): .osc lexer/compiler, varlist, constfile, .hof, timetables,     │
│            ticket packs (.otp), money (.cfg), sound.cfg, model.cfg [scripttexture]   │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

**Ownership.**

| Layer | Crate | Owns | Knows nothing about |
| :--- | :--- | :--- | :--- |
| Formats | `opts-core` | parsing of every file the runtime reads; the `.osc` compiler stays in `opts-sim` as today [K] but its diagnostics are a format concern | time, vehicles, Bevy |
| Script runtime | `opts-sim` | `ScriptEngine` (VM), `ConsistScriptHost`, `CallbackRegistry`, `InputTable`/`OutputTable`, the service **traits**, the event queue, budgets, determinism | the ECS, rendering, audio playback, file lookup |
| Bindings | `opts-bevy` | service **implementations** (canvas upload, font registry, hof/timetable adapter, ticket and money adapter, ground query), the per-frame feed and read-back, trigger sources, the session save of script state | the script language |
| Game | `opts-game` | which consists are scripted and at which LOD (D8), line and course selection UI, control-protocol methods, the headless rig | VM internals |

**Communication rules.** `opts-sim` exposes traits; `opts-bevy` implements them and hands `Arc<dyn Service>` objects to the host at spawn. The host never calls back into Bevy. Data flows in two directions only: the input feed before a step and the output read-back after it; everything else (events, diagnostics, canvas dirt) is a queue the binding drains.

**Why a host per consist (D1), restated for the implementer.** OMSI's unit of script state is the *part* (one engine, one variable store), but doors, passengers, displays and cables are chain-wide. The host owns the parts, runs them front to back, bridges the cables and routes triggers; the game sees one object per consist.

## 🔌 The Host Interface

### Callback registry (D3)

Every `(M.V.name)` goes through a `CallbackRegistry` keyed by lower-case name. An entry is a **signature** plus an **implementation**:

```rust
struct CallbackSignature {
    float_args: u8,        // popped from the float stack, bottom to top as in callbacks.md
    string_args: u8,       // popped from the string stack
    float_results: u8,     // pushed after the call
    string_results: u8,
    confidence: Confidence,// High | Medium | Low, from callbacks.md
}
enum CallbackImpl {
    Service(Box<dyn Fn(&CallbackCtx, Args) -> Results + Send + Sync>),
    Stub,                  // pops and pushes neutral values; reported once per (vehicle, name)
}
```

Rules:

1. **The registry, not the implementation, moves the stacks.** The VM pops `float_args` and `string_args`, calls the implementation with them, and pushes exactly `float_results`/`string_results`. An implementation cannot leave the stacks misaligned, and a stub is stack-correct by construction. This replaces today's "handler receives both stacks" closure [K: `ExternalHandler`].
2. **Neutral stub values** follow the catalogue's sentinel conventions: index results `-1` (what the scripts test for "not found", `IBIS-2.osc:147-157`), counts `0`, strings `""`, `GetTTDelay` `0`, `GetFontIndex` `0` ("default" fallback), `GetHeightAbovePoint` `0`, canvas calls nothing.
3. **Stubs are honest** (strategy principle 2): the first call per (vehicle, callback) logs `ERROR` with vehicle, script, block and callback; the integration report counts calls per callback and per status.
4. **Unregistered names** (none of the 39) are treated as a stub with zero arity and logged, as today.
5. **Hot path:** names are interned at compile time [K: `Names.externals`]; the registry is resolved to a per-program table of indices at engine creation so a call is an array lookup.

**Full callback list and target status** (implementation = the service the catalogue names; status after the phase in brackets):

| Group | Callbacks | Service | Status today → target |
| :--- | :--- | :--- | :--- |
| Randomness | `NrSpecRandom` | `RandomService` (seeded per vehicle) | done [K] → keep; seed from the stable vehicle id (Variable System, Determinism) |
| `.hof` tables | `GetTerminusString/Index/Code`, `GetRouteIndex`, `GetRouteTerminusIndex`, `GetRouteBusstopIdent`, `GetBusstopCount/Index/String`, `GetDepotStringGlobal` | `RouteService` | done [K], static context → live (P3) |
| Timetable | `GetTTLineString`, `GetTTTerminusIndex`, `GetTTBusstopCount/Index/Name/Dep/Arr`, `GetTTDelay` | `TimetableService` | done [K] over static values → live course state (P3) |
| Fonts and text | `GetFontIndex`, `TextLength` | `FontService` | done [K] → cached handle table (Display binding) |
| Canvas | `STNewTex`, `STLock`, `STUnlock`, `STFilter`, `STSetColor`, `STDrawRect`, `STTextOut`, `STReadPixel`, `STGetR/G/B`, `STLoadTex` | `CanvasService` | **missing** → stack-correct stubs (P0), real (P4) |
| Tickets and cash | `GetTicketName`, `GetTicketValue`, `GiveChangeCoin` | `TicketService`, `CashService` | done [K] (data) → effects on passengers (P5) |
| Passengers | `GetHumanCountOnSeat`, `GetHumanCountOnPathLink` | `PassengerService` | done [K] (returns 0) → counts (P5) |
| Geometry | `GetHeightAbovePoint` | `GroundService` | done [K] ❓ frame (E4) |

### Variable interface

The host reads and writes script variables only through two generated tables (D2):

```rust
struct InputRow  { name: &'static str, unit: Unit, source: Source, rate: Rate, parts: PartScope }
struct OutputRow { name: &'static str, unit: Unit, consumer: Consumer, parts: PartScope }
```

`Source` ∈ {Physics, Clock, Weather, Environment, Passengers, Timetable, Repaint, Constant, Planner}; `Consumer` ∈ {Physics, Lights, Animation, Display, Audio, Passengers, Timetable, Planner, Debug, None}; `PartScope` ∈ {Head, EveryPart, PartsWithAxles}. The tables are the machine-readable form of [`predefined-variables.md`](../formats/text/vehicles/predefined-variables.md) plus the catalogue's "Variables" lines; the feed and the read-back are generated from them, and the rig checks that every name a set reads is fed, declared or an accepted zero (strategy D2).

### Trigger and event interface

```rust
enum TriggerSource { Key, HotSpot { mesh: String, phase: Press | Drag | Release }, Engine, Script }
struct TriggerRequest { part: PartSel, name: String, source: TriggerSource }
enum PartSel { Head, All, Owning }       // Owning = the part(s) that define {trigger:name}
struct ScriptEvent { part: PartId, kind: EventKind, name: String, payload: Option<String> }
```

A trigger run may leave a float on the stack; the host exposes it as `TriggerResult { value: Option<f32> }` for the two engine-raised triggers that return one (`malfunction_gettime` [C: `MAN_GN92_main.osc:155-160`]). `ScriptEvent.payload` is the top string of the string stack at the raise for `(T.F.*)` events (announcement file names [C: `IBIS-2.osc:681`]) and `None` otherwise.

## 📊 Variable System

### Stores

One `VariableStore` per **part**: float slots and string slots addressed by interned index, as today [K: `variables.rs`]. A consist holds:

* one store per part with its own scripts;
* for a `[scriptshare]` part, **no store of its own**: it aliases the store of the part it shares with (the GN92 trailer [C]). This is the wiki's rule 📚 and the strategy's D1; the alternative "private copy per part" (strategy §10 alternative 3, from the route document) is kept as a flag until Q-OSC-R2 is settled.

### Population at load

1. Compile the set; intern every name by kind.
2. Declare the names of every `[varnamelist]` and `[stringvarnamelist]` file; the union is the declared set. Names used but not declared are checked against the predefined lists; the rest are reported once per vehicle (today's behaviour [K: `undeclared_variables`]).
3. Load every `[constfile]` into the part's constant table; later files override earlier ones (Q-CST-002 assumed last-wins); curves are sorted and held at the ends (Q-CST-001).
4. Apply **pre-init values** in this order: predefined defaults from the input table (`Pause` 0, `AI` 0/1, …), repaint `[setvar]` of the chosen livery [K, exists], session `[setvar]` overrides, then — for a restored session — the saved variable snapshot.
5. Run `{init}` of every part, front to back.

### Three kinds of variable, three rules

| Kind | Who writes | Rule |
| :--- | :--- | :--- |
| Engine input | the feed, before the step | written to every part that declares or reads it (axle and wheel families only to parts with axles) |
| Script state | the scripts | never touched by the host except by the session restore |
| Engine output | the scripts; read by the read-back after the step | read **after the whole frame of the consist**, never between parts or macros, because scripts accumulate into outputs (`M_Wheel` [C: `antrieb_D854_GN.osc:495`]) |

Names that are both (`Throttle`, `Brake`, `AI_Scheduled_AtStation`): the feed writes them only for the player's head part when the driver owns the pedals; when the planner or the script owns them (AI, `{frame_ai}`), the feed leaves them alone. The ownership is one flag per consist (`PedalOwner ∈ {Driver, Planner, Script}`), which fixes research report §5 item 3.

### Serialization

The session save stores, per part: the ordered list of (name, value) for non-zero floats and non-empty strings [K: `export_variables`, used by `session_vehicles/state.rs`], plus the random-generator state, the accumulated `GetTime`, the canvas state (Display binding) and the per-part trigger latches. Restore writes the snapshot *before* `{init}`? No: **after** `{init}` would re-run start-up logic over restored state, **instead of** `{init}` loses what `{init}` computes from constants. Rule: restore runs `{init}`, then overwrites with the snapshot, then runs one `{frame}` with `Timegap` 0 so derived values settle. Scripts that integrate time (`GetTime` comparisons) see the saved clock.

### Determinism

* All time reaches scripts through `Timegap` and `GetTime` set by the host from the fixed step; `GetTime` is accumulated in `f64` and cast per read (fixes research report §5 item 5).
* `random` and `NrSpecRandom` use one generator per part seeded from the **stable vehicle id** of the session registry (not `Entity::to_bits`, item 8), reseeded on restore from the snapshot.
* The engine inputs of a step are those of the previous physics step (Q-BSI-2 ❓; the order is recorded, not guessed: the rig logs which inputs changed between feed and read-back).

## ⚙️ Execution Model

### One consist step (FixedUpdate, after physics)

```
for each scripted consist within the LOD radius (D8):
  1. feed:      InputTable → every part's store
  2. triggers:  drain the consist's trigger queue in arrival order; each runs as a
                top-level run on the owning part(s); collect TriggerResult values
  3. frame:     parts front to back: run {frame} (or {frame_ai} when the part is AI-driven)
                ─ after each part: propagate control cables (Control cables, below)
  4. read-back: OutputTable ← stores; events ← queue; canvases ← dirty flags
  5. budget:    account instructions and µs per part; trip the LOD valve on overrun
```

Rules carried from the guide (each is corpus- or code-backed there):

* A top-level run starts with empty stacks and a zero register file; registers are shared by the run and all macros it calls.
* No yield, no coroutine: a block runs to its end. A script cannot wait; the host provides no scheduler primitives beyond "next step".
* Several `{frame}` blocks of one part run in `[script]` order; macros are global per part.
* Events raised during a run are queued with the part id and payload and delivered after step 4; nothing is dropped (D6).
* `$msg` **leaves its string on the stack** (corpus rule, Q-OSC-G1): the VM change is one line and makes 78 corpus sites right; the console line is still emitted.

### Trigger routing (strategy D1, Q-BSI-5)

| Source | Routed to |
| :--- | :--- |
| Key binding (`keyboard.cfg`, `gamectrler.cfg`) | the head part only (measured on the NF6D 2026-10-21: the car behind has a cab of its own that answers `automatic_D` and marks the head inactive; the cars hear the head through the control cables) |
| Mouse hot-spot | the part whose mesh was hit |
| Engine (`collision`, `malfunction_*`, `ai_scheduled_settarget`, `veh_tank`) | the head part, plus every other part that defines it |
| Script `(T.L.name)` where `{trigger:name}` exists in the same part | **configurable**: `ScriptRaiseRunsTrigger` on or off per vehicle (default off, as the VM behaves today; Q-OSC-G2 decides the default). Either way the event is still delivered. |

### Control cables (consist state, D1)

A `[control_cable_front]`/`[control_cable_back]` entry declares a pair of variable names on two adjacent parts [C: `NF6D_B.bus`]; the tram's A part declares 102.

**As built (2026-10-21, `opts_sim::script::consist`)**: the entry reads `kind`, `number`, *first name*, *second name*, *coupling variable*. The scripts of the NF6D write the **second** name (`SLA_Elec_Bat`, `SLW_BlinkerL_A`) and only read the **first** (`SLAR_Elec_Bat`, `SLR_BlinkerL_A`; no script writes the first), so the second is what a unit puts on the line and the first is where the line's value arrives (the wiki's "read"/"write" wording is ambiguous on this). Every unit of a run of units the line connects (each declares the cable and the plugs between neighbours have their coupling variable above 0.5; a unit without scripts passes it on) receives the value of the largest magnitude on the line, **its own included**: a head with nothing in front of it gets its own battery switch back, and the cab at one end drives the cars at the other. The copy runs once per step after all engines ran (head first, then the parts front to back), so a value takes one step per hop at most. Which of the original's possible orders this matches is untested (Q-VS-1); the NF6D's behaviour (battery, cab activation, door commands to B and C, traction) is the check. `vehicle.script_vars unit=n` reads the engine of the n-th unit.

### LOD (D8)

Full sets run for the player's consist and consists within the radius; parked and far vehicles keep their stores and are stepped by the idle model (no `{frame}`, inputs frozen). Crossing the radius inwards runs one `{frame}` with `Timegap` equal to the elapsed time, capped at one second, so timers settle without a jump.

## 🔗 Subsystem Bindings

### Physics binding (interface of D4)

The scripts and the physics exchange values only through the tables; the wheel model itself is M-0097's design, not this sheet's.

| Direction | Variables | Rule |
| :--- | :--- | :--- |
| in | `Velocity`, `Velocity_Ground` (km/h), `A_Trans_X/Y/Z` (m/s² in vehicle axes), `n_Wheel`, `Wheel_RotationSpeed_n_s` (rpm, **from the wheel model, not from chassis speed**), `Wheel_Rotation_n_s` (rad), `Axle_Steering_n_s` (rad, sign ❓ item 10), `Axle_Suspension_n_s` (m), `articulation_n_alpha/beta` | fed to parts with the axles; `articulation_*` has one writer (the consist physics) |
| out | `M_Wheel` (N·m at the driven wheels, `M_WHEEL_UNIT` calibrated by E1), `Brakeforce`, `Axle_Brakeforce_n_s`, `Axle_Springfactor_n_s`, `Throttle`/`Brake` when the script owns the pedals | read after the frame; the three-way fallback of D4 (script torque → const-file curve → constants model) chooses the source per consist and logs the choice once |
| hold | parking and stop brake | a non-zero script brake at standstill holds the vehicle: the zero-speed clamp uses the **script's** brake, not the driver's (fixes item 1) |

Acceptance belongs to P1 (strategy table); this sheet only fixes the interface so that the catalogue's drivetrain and brake functions have every input and output wired.

### Audio binding (events; the engine is M-0102)

The runtime delivers `ScriptEvent`s; it does not play sounds. The binding:

* matches `(T.L.name)` against the vehicle's `sound.cfg` `[trigger]` entries (90.3 % match [S]) and hands the matched sound ids to the audio layer with the part's transform;
* passes `(T.F.name)` with its payload (a `.wav` path relative to the vehicle, resolved case-insensitively; the GN92 builds `..\..\Announcements\…` [C]) to the announcement channel;
* reads the `Snd_OutsideVol`, `Snd_Microphone`, `Snd_Radio` outputs every step as mixer parameters;
* counts unmatched events per name in the integration report (the engine-raised candidates of Q-BSI-4 appear there as *never raised by a script*).

Until M-0102 lands, the binding records every event with its payload in the rig and the scene trace, so sound coverage is measurable before anything is audible.

### Display binding (canvas, D5)

```rust
trait CanvasService {
    fn new_tex(&mut self, part: PartId, tex: u32);           // STNewTex: allocate at [scripttexture] size, clear
    fn lock(&mut self, part: PartId, tex: u32);               // STLock: begin a draw batch
    fn unlock(&mut self, part: PartId, tex: u32);             // STUnlock: end the batch, mark dirty
    fn filter(&mut self, part: PartId, tex: u32);             // STFilter: sampling mode flag (nearest/linear) ❓
    fn set_color(&mut self, part: PartId, tex: u32, argb: [u8; 4]);
    fn draw_rect(&mut self, part: PartId, tex: u32, x1: i32, y1: i32, x2: i32, y2: i32);
    fn text_out(&mut self, part: PartId, tex: u32, x: i32, y: i32, font: FontHandle, a: i32, b: i32, text: &str);
    fn read_pixel(&mut self, part: PartId, tex: u32, x: i32, y: i32); // stores the last pixel
    fn last_pixel(&self, part: PartId, tex: u32) -> [u8; 4];          // STGetR/G/B read it
    fn load_tex(&mut self, part: PartId, tex: u32, file: &str) -> bool; // STLoadTex ❓ (E5)
}
trait FontService { fn font_index(&mut self, name: &str) -> FontHandle; fn text_length(&self, font: FontHandle, text: &str) -> u32; }
```

Design points:

1. **One CPU canvas per `[scripttexture]` block of the part** running the script (the texture index is the block's position [S: `callbacks.md`]); allocated lazily on the first `STNewTex`; size from `model.cfg` [K: `ScriptTexture` parsed]. A mesh binds a canvas through `[useScriptTexture] n` or a `\S:n` material name ❓.
2. **Drawing is immediate on the CPU canvas**; the GPU `Image` is updated only for canvases marked dirty by `STUnlock`, at most `CANVAS_UPLOADS_PER_FRAME` (8, as for text textures [K: `text_texture.rs`]) per frame, oldest dirty first, the rest the next frame. A canvas drawn every frame (the matrix transfers pixels gradually, `VMatrix.osc:468-479` [C]) therefore uploads at most once per frame.
3. **Fonts** come from the existing `FontRegistry` [K]; `GetFontIndex` is answered from a per-vehicle **cache keyed by the literal name**, because the GN92 calls it eight times *every frame* [C: `VMatrix.osc:157-164`]; unknown names map to the engine fallback `default` (handle 0) and are reported once.
4. **Undecided semantics have named flags with the common case exact** (strategy F7): `STTextOut` `a`/`b` (E2; the `0 0` case draws left-aligned, unclipped), `STSetColor` order and alpha (E3; assumed `tex A R G B`, alpha blends), `STLoadTex` path base and return value (E5; assumed vehicle texture folder, consumes the string, no result), `STFilter` (assumed sampling mode).
5. **Save state:** the canvas pixels of each part are part of the session snapshot (Variable System, Serialization), compressed; without them a restored bus shows a blank board until the script redraws, which many scripts do only on a route change.

### Route data binding (P3)

```rust
trait RouteService   { /* the ten .hof callbacks over a HofFile */ }
trait TimetableService {
    fn line_string(&self) -> &str;             // GetTTLineString
    fn terminus_index(&self) -> i32;           // GetTTTerminusIndex (hof row of the course's terminus)
    fn stop_count(&self) -> u32;               // GetTTBusstopCount
    fn current_stop(&self) -> u32;             // GetTTBusstopIndex (0-based, advances as the bus serves stops)
    fn stop_name(&self, n: u32) -> &str;
    fn departure(&self, n: u32) -> u32;        // seconds of the day
    fn arrival(&self, n: u32) -> u32;
    fn delay_seconds(&self, clock: SimClock) -> f32; // positive = late
}
```

The services are **data objects owned by the consist** and updated by the game, not by the callbacks:

* The `.hof` is chosen as today (the depot file that applies to the vehicle) [K: `vehicle.rs` lookup]. It is immutable.
* The **course state** (`line`, `course`, `current_stop`, the stop list with times) is set by the line and course selection UI or the control protocol, or by the dispatcher of a scheduled bus through the engine trigger `ai_scheduled_settarget` after setting `AI_Target_Index` and `SetLineTo` [C: `VMatrix.osc:38-60`].
* **Read-back closes the loop:** after each frame the binding reads the string `act_busstop` and the float `target_index_int`; a changed `act_busstop` moves `current_stop` to that ident's position in the course (the scripts advance it with `IBIS_vor` [C: `IBIS-2.osc:271-282`]) and informs the passenger layer which stop the doors serve; `Refresh_Strings` ≠ 0 marks the display strings as changed for the `[texttexture]` path and is reset to 0 by the binding. `schedule_active` is fed as 1 while a course is set.
* `delay_seconds` is computed from the simulation clock against the course's time at `current_stop` every step, which turns `GetTTDelay` from a constant into the live value the scripts switch "too early/too late" textures on [S: `callbacks.md` notes].

### Ticket data binding (P5 for the effects)

```rust
trait TicketService { fn ticket_name(&self, idx: u32) -> &str; fn ticket_value(&self, idx: u32) -> f32; fn count(&self) -> u32; }
trait CashService   { fn give_change_coin(&mut self, part: PartId, coin_idx: u32); }
```

Data from the vehicle's ticket pack `.otp` and the currency `Money/<cur>.cfg` [S: `ticketpacks.md`, `money.md`; K: implemented in the context]. The read-back watches `GivenTicket` (≥ 0 = a ticket index handed over this step, reset to −1 by the script [C: `ticketprinter.osc:251, 347`]) and `GiveChangeCoin` calls, and forwards both as **sale events** (`TicketGiven { part, ticket }`, `CoinGiven { part, coin }`) to the passenger and economy layers. Fare computation stays in the scripts (the Urbino's fare module reads timetable times [C]); the host only answers prices.

### Lights, animation, text textures

Unchanged consumers [K: `vehicle_lights.rs`, `vehicle_anim.rs`, `vehicle_text.rs`], moved from "lock the engine per mesh per frame" (research report §5 item 13) to reading the output snapshot of step 4 of the consist step: the read-back copies the variables the consumers subscribed to into a plain `OutputSnapshot` per consist, and the Bevy systems read that without locking.

## 🔥 Hot Reload

Scripts, varlists and constant files of a loaded vehicle can be reloaded at runtime (the game already watches vehicle directories [K: `hot_reload`]). On a change of any file of a set:

1. The set is recompiled into a new `Program`; on a compile error the old program stays and the error is reported (no partially applied set).
2. The new program is **linked against the existing stores**: variables keep their values by name (new names start at 0, removed names are dropped), constants are reloaded, the random state and clock continue.
3. `{init}` is **not** re-run (it would restart the engine and reset the IBIS); a developer can request it through the control protocol (`vehicle.script_reinit`).
4. Canvases are kept; the next `STNewTex` of the script clears them.
5. Triggers queued during the swap are delivered to the new program.

What resets: compiled block indices, interned ids (the stores are re-mapped by name), the per-site "reported once" sets, so diagnostics show up again. What survives: every variable value, the canvases, the course state, the physics.

## 🐛 Error Handling and Recovery

| Condition | Handling | Visible as |
| :--- | :--- | :--- |
| Compile error in one file | the file's blocks are disabled, the rest of the set runs (today [K]) | `ERROR` per file with line; integration report |
| Compile warning (leniency) | applied as `osc.md` specifies | `WARN` once per site; counted |
| Stub callback called | neutral results, stack-correct | `ERROR` once per (vehicle, callback); report |
| Stack underflow | 0 / `""` substituted | `WARN` once per site; report |
| Macro depth exceeded | call ignored | `WARN` once |
| NaN or ∞ in an output | output replaced by its last finite value; the input feed never carries NaN | `WARN` once per (vehicle, output); P0 acceptance: 0 over 600 frames |
| Budget overrun (Performance) | consist demoted to idle LOD for 1 s, then retried | `WARN`, report |
| Missing script file | warning, set loads without it (today) | `WARN` |
| Panic inside a service | caught at the registry boundary; the callback becomes a stub for that vehicle | `ERROR` once |

**Isolation.** A failure in one consist never affects another: each has its own stores, queues, budget and reported-once sets. A failing part inside a consist is run with its last outputs frozen; cables from it carry the frozen values.

**User-visible state.** The loading screen's problem list shows per vehicle: compile errors, stub callbacks hit, missing files; the F10 panel shows per consist: parts, instructions per step, budget share, stubbed callbacks, unfed inputs, last diagnostic. The control protocol exposes the same (`vehicle.script_report`).

## 🧪 Debugging Support

Required for developers (the rig is the primary surface, strategy principle 8):

* **`script_rig`** (headless, `opts-sim` example): loads a set, applies a scripted input profile (pedals, speed ramp, triggers, line and course), runs N steps, writes a CSV of chosen variables per step and the **integration report** (unfed inputs, unconsumed outputs, callbacks by status, events raised with payloads, cable copies, diagnostics, budget). Corpus mode over all `.bus` files.
* **Variable watch** through the control protocol (`vehicle.script_vars`, exists [K]) extended with `set` for floats and strings and with per-step tracing of a name list (`vehicle.script_trace name1,name2 → CSV`).
* **Block tracing**: an opt-in per-vehicle trace of top-level runs (block name, instructions, stack depth at `{end}`, events raised), bounded by a ring buffer; `script_diag` disassembly stays.
* **Trigger injection**: `vehicle.trigger name` (exists [K]) extended with the part selector.
* **Canvas dump**: `vehicle.canvas part tex → PNG` for golden images (P4 acceptance).

Runtime-only (ship with the game): the F10 panel counters and the problem list above; the scene trace records the vehicle's stubbed callbacks and unmatched events per session.

## ⚡ Performance

Budgets from the strategy (§7), restated as rules:

* **Per consist per step:** instructions and wall time are measured; soft budget 1 ms for a 23-part consist, 0.2 ms for a solo bus; overrun handling as in Error Handling.
* **Per frame:** at most 8 canvas uploads and 0.5 ms of canvas work; the rest waits.
* **Interning:** no string hashing on the hot path: callbacks, inputs, outputs and cables are resolved to indices at load; the feed and read-back iterate index lists generated from the tables.
* **Caching:** `GetFontIndex` by name (per vehicle), `TextLength` by (font, string) with a small LRU (the matrix measures the same string every frame), `.hof` lookups by code and ident through the existing indices [K: `build_indices`].
* **Avoided stalls:** no file I/O in a step (`STLoadTex` reads through a preloaded texture cache keyed by path; a miss is loaded on a worker and drawn the next step), no allocation per instruction (stacks and registers pre-allocated per part), no engine lock from render systems (the output snapshot of the lights and animation binding).
* **Scale:** D8's radius bounds the number of full sets; the AI one-script path (P7) is the cheap model for the rest.

## 🔀 Alternatives Considered

1. **Keep the "handler gets the stacks" callback interface and implement the 12 canvas calls in it.** Rejected: every implementation must get the arity right by hand, a stub cannot be generated, and the research report's finding that a wrong arity corrupts the run would stay a permanent risk. The signature-first registry makes arity a data error caught by a contract test.
2. **Render script textures on the GPU (draw commands recorded per call).** Rejected for now: `STReadPixel`/`STGetR` read pixels back inside the same frame (the GN92 reads a bitmap pixel by pixel to build the matrix [C: `VMatrix.osc:457-479`]); a CPU canvas answers that synchronously and the upload budget keeps the cost bounded.
3. **One variable store per consist with merged names.** Rejected (strategy §10.3): OMSI keeps private stores per part and exchanges state through cables; the tram's 102 cables are meaningless in a merged store.
4. **Run scripts on a worker thread, one frame behind.** Rejected: triggers from the driver and physics inputs would lag a frame, and the read-back order (outputs after the whole frame) is simpler to guarantee in place; the median set costs under 1 % of a frame.
5. **Run `{trigger:x}` on every script raise, as `osc.md` claims.** Deferred, not rejected: no surveyed set depends on it and the VM does not do it; it stays a per-vehicle flag until Q-OSC-G2 is settled by experiment, so neither reading is hard-coded.
6. **Re-run `{init}` on hot reload.** Rejected: it restarts the engine and resets the IBIS; relinking by name keeps the driving state and is what a script author expects when editing a display macro.

## ❓ Open Questions

| ID | Question | Evidence that would resolve it |
| :--- | :--- | :--- |
| Q-OSC-R1 | `$msg` keeps or pops its string (default here: keeps) | 78 of 325 corpus sites use it afterwards; OMSI experiment Q-OSC-G1 |
| Q-OSC-R2 | `[scriptshare]` parts alias the store or hold a private copy | NF6D has no scriptshare; find an articulated bus whose trailer scripts *write* a shared name and check in OMSI whether the head sees it |
| Q-OSC-R3 | Does a script raise run the same part's trigger (default here: off) | Q-OSC-G2 experiment; the 253 script-only pairs of the research report |
| Q-OSC-R4 | Cable propagation order and latency (front-to-back then back-to-front) | NF6D rear-cab drive in the rig against the expected one-step latency; E8 |
| Q-OSC-R5 | Which part a key trigger reaches when several define it | E8 (`NF6D` C-car door) |
| Q-OSC-R6 | Value read after a returning trigger (`malfunction_gettime`) | E7 session, repair menu timing |
| Q-OSC-R7 | `STTextOut` a/b, `STSetColor`, `STLoadTex`, `STFilter` semantics | E2, E3, E5 |
| Q-OSC-R8 | Are inputs of a step the previous physics step's values (Q-BSI-2) | rig: scripts that read before they write; compare `Velocity` with the physics trace |
| Q-OSC-R9 | Save-state size of canvases per vehicle; compress or redraw | measure on the GN92 (two 256×32 canvases ❓ size from `model.cfg`) |
| Q-OSC-R10 | `M_WHEEL_UNIT`, `Axle_Steering` sign, `A_Trans_*` definition | E1, physics trace (owned by M-0097) |

## 🔁 Reconciliation with the prior research

Read again after the three documents were written (2026-10-16):

* **Agrees:** every decision D1-D8, the phase order, the demand profile, the callback signatures of `callbacks.md`, the fifteen suspicious spots of the research report (five of them get a concrete rule here: pedal ownership, `GetTime` in `f64`, the stable seed, the script-brake zero clamp, the output snapshot instead of per-mesh locks).
* **Extends:** the callback registry is signature-first instead of a handler closure; events carry a payload; triggers carry a part selector and a return value; the load order of pre-init values, the restore rule and the hot-reload relink are new; the cable propagation has an order; `GetFontIndex` and `TextLength` are cached.
* **Corrects by date, not by contradiction:** the research report's "41 of 42 callbacks unimplemented" and the strategy's "only `NrSpecRandom`" describe `f531a3e`; M-0093 (same day, later) implemented 27, so P0's remaining stub work is the 12 canvas calls. The route document's "registers block-local" (also in `script-vm.md`) is contradicted by the corpus and the code (guide, Execution Model); this sheet follows the code.
* **Contradicts, and says why:** `osc.md` stated that `(T.L.name)` runs `{trigger:name}`; the VM does not, no surveyed set depends on it, and the sheet keeps both readings behind a flag (Q-OSC-R3). The VM's popping `$msg` is treated as a defect on corpus evidence (Q-OSC-R1); the strategy did not know of it.

## 📝 Provenance

* **Prior documents read in full:** strategy, research report, M-0093 and M-0096 to M-0103, the six format specs, `script-vm.md`, `vehicle-scripts-route.md`, `articulation-physics.md`, ADR-0002, `standards.md`.
* **Code read (not modified):** `crates/opts-sim/src/script/*` (compiler, VM, strings, variables, loader), `crates/opts-bevy/src/{host_callbacks,vehicle_script,text_texture}.rs`, `crates/opts-omsi/src/formats/vehicles/hof.rs`, `crates/opts-game/src/in_game/omsi_world/vehicle.rs` (script attachment), `session_vehicles/state.rs`.
* **Corpus read (not committed):** the complete script sets of `MAN_NL_NG/MAN_GN92_main.bus` (+ trailer), `MAN_SD200/MAN_SD83.bus`, `EMT_Citaro/EMT_Solo.bus`, `Tram_NF6D/NF6D_A.bus` (+ B, C), `Urbino_II/SU_12.bus`; a corpus-wide scan of `$msg` sites; `program/varlist_*.txt`.
* **Runs:** `script_diag` on the five sets (read-only).

**Clean-room attestation (ADR-0002):** no OMSI binaries were inspected; no GPL source was read; no OMSI assets were committed.

## 🧭 Deferred (named, not designed)

| Item | Why deferred | Owner |
| :--- | :--- | :--- |
| AI vehicle scripts near the player (`{frame_ai}` with planner pedals, LOD switch) | needs the physics contract first; the one-script path is a separate design | [M-0103](../roadmap/phase-4/M-0103.md) |
| The audio engine (mixing, spatialisation, `sound.cfg` playback) | depends on the audio library decision (strategy §11.3); this sheet only delivers events with payloads | [M-0102](../roadmap/phase-4/M-0102.md) |
| Passenger simulation (boarding through `PAX_*`, counts on links and seats, cash and ticket hand-over) | needs the cabin graph and the consist-wide index space (E6); this sheet only delivers the sale events and the door flags | [M-0101](../roadmap/phase-4/M-0101.md) |
| Rigid-body train physics for the tram's torque and PZB inputs | rail dynamics are their own model; no signalling input exists | [Rail vehicles](../simulation/rail-vehicles.md), M-0098 for the consist state |
| Force feedback device binding (`FF_*`) | no milestone; outputs are read into the snapshot so a binding can consume them later | none |

## 🔗 Related Documents

- [Bus script integration strategy](bus-script-integration.md) — decisions and phases this sheet implements
- [Research report](../meta/bus-script-integration-2026-10-14.md) — demand numbers and defects
- [OMSI scripting guide](../formats/text/vehicles/osc-guide.md) — language semantics the execution model follows
- [Vehicle functions](../simulation/vehicle-functions.md) — the demand this sheet serves
- [Script VM](script-vm.md), [Callbacks](../formats/text/vehicles/callbacks.md), [Predefined variables](../formats/text/vehicles/predefined-variables.md), [`.hof`](../formats/text/vehicles/hof.md), [Timetables](../formats/map/timetables.md), [Ticket packs](../formats/text/ticketpacks.md), [Money](../formats/text/money.md)
- [OMSI experiment protocol](omsi-experiment-protocol.md) — E1 to E8
- Deferred designs: [M-0101](../roadmap/phase-4/M-0101.md) doors, passengers and tickets, [M-0102](../roadmap/phase-4/M-0102.md) sound and force feedback, [M-0103](../roadmap/phase-4/M-0103.md) AI scripts near the player, [Rail vehicles](../simulation/rail-vehicles.md) for train physics

