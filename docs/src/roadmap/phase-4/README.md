# 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)

*Goal: Build the transit simulation logic and the game binary.*

**Status:** 🟡 In progress — physics, world streaming, script VM and script-driven vehicles, AI traffic, walking pedestrians and articulated buses running; passengers, timetables, audio, map-streaming/terrain specs open

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0022 — Vehicle Physics & Kinematics Scaffold (`opts-sim`)](M-0022.md) — 🟢 Complete (physics spec & simulation core)
- [M-0023 — Bevy Vehicle ECS Simulation Bridge & Interactive Driving (`opts-bevy`)](M-0023.md) — 🟢 Complete (ECS physics bridge, input mapping, transform sync & interactive driving)
- [M-0024 — OMSI Map Tile Grid & Chunk Streaming Engine (`opts-game`, `opts-bevy`)](M-0024.md) — 🟡 Partly delivered in `opts-game`; spec, async jobs, extraction open
- [M-0025 — Terrain Heightmap & Water Surface Mesh Generation (`opts-core`, `opts-bevy`)](M-0025.md) — 🟡 Terrain mesh and sampling delivered; spec, water, ground-height query open
- [M-0026 — OMSI Script Stack VM & Dynamic Variable System (`opts-sim`)](M-0026.md) — 🟢 Complete for vehicles (compiler, VM, variables, passive bridge); audio events wait for M-0027
- [M-0027 — Vehicle & Environmental Spatial Audio Simulation (`opts-bevy`)](M-0027.md) — 🟡 Engine implemented 2026-10-20 (voices, buses, settings, `audio.*`); sounds follow in M-0139, M-0140, M-0102
- [M-0139 — Sounds of Scenery Objects and Splines](M-0139.md) — 🟡 First slice 2026-10-20 (continuous and triggered scenery sounds live; conditions, eviction open)
- [M-0140 — Sounds of AI Traffic](M-0140.md) — 🟡 First slice 2026-10-20 (AI engine, rolling and wheel sounds live; doors, horns, brakes open)
- [M-0037 — Shipped-Work Reconciliation (Roadmap, Phase Pages, Board)](M-0037.md) — 🟢 Complete (audit 5037f89a F-019, F-023)
- [M-0038 — Scrambled-Mesh Completion (`opts-core`, `opts-bevy`)](M-0038.md) — 🟡 Engineering complete (async decoding, parameters, cache policy); generator research open
- [M-0039 — Text Textures for Signs, Stops and Displays](M-0039.md) — 🟢 Complete (`[texttexture]` rendering for static strings)

- [M-0045 — Script-Driven Vehicle Animation, Lights and Doors](M-0045.md) — 🟡 Animation, visibility, lights, key and hot-spot triggers done; in-game screenshot check open
- [M-0046 — World Loader in `opts-bevy`](M-0046.md) — ⏳ Planned (extraction refactor)
- [M-0049 — AI Road Traffic (`opts-traffic`, `opts-bevy`)](M-0049.md) — 🟡 Running in game (lights, right of way, lane changes); debug tooling, speed limits, scheduled AI open
- [M-0050 — Pedestrians](M-0050.md) — 🟡 Walking pedestrians done; crossings, stops and boarding open
- [M-0080 — Tile Loading Controls: Load All Tiles and Tile Radius](M-0080.md) — 🟡 core delivered 2026-10-18 (settings, live streaming, `scene.tiles`); wizard switch, memory guard and page label open (R-20, R-21)
- [M-0084 — Viewpoint Editor (Per-Vehicle Camera Positions)](M-0084.md) — ✅ Done (R-26)
- [M-0089 — Tile Streaming Performance: Investigation and Improvements](M-0089.md) — 🟢 Complete 2026-10-18 (worst frame 36.9 ms), 🔴 high (R-32)
- [M-0093 — Host Callbacks — Timetable, Termini, Bus Stops, Tickets (IBIS)](M-0093.md) — 🟢 Complete (2026-10-14)
- [M-0094 — Script Textures](M-0094.md) — 🟢 Complete (2026-10-14)
- [M-0095 — AI Vehicle Profiles](M-0095.md) — ✅ Implemented 2026-10-14 (phase 1: profile layer and formulas; measured characterisation through the script VM and the dynamic mod
- [M-0096 — Script Rig, Input Table and Callback Safety Net](M-0096.md) — 🟢 Complete 2026-10-18, 🔴 critical
- [M-0097 — Script Physics Contract: Torque, Brakes and Wheels](M-0097.md) — 🟡 Implemented 2026-10-18, units from the scripts 2026-10-19, SD200 acceleration range open, 🔴 critical
- [M-0098 — Consist Script State: Per-Part Engines, Script Share and Control Cables](M-0098.md) — ✅ Done 2026-10-21
- [M-0099 — Route, Timetable and IBIS Data Layer](M-0099.md) — 🟢 done 2026-10-19
- [M-0100 — Script Textures: Canvas, Fonts and the `ST*` API](M-0100.md) — 🟡 Partly built; glyph text 2026-10-21; NG272 and HH109 destination boards show 2026-10-26
- [M-0101 — Doors, Passengers, Tickets and Cash](M-0101.md) — 🔴 Not started (proposed 2026-10-14)
- [M-0146 — Interior Rattling: the Road Shakes the Bus](M-0146.md) — ✅ Implemented 2026-10-26 (project lead's request): `A_Trans_Z`, `.surf` height maps and surface types under every wheel
- [M-0141 — Vehicle Controls and Systems: the Driver's Workplace Works](M-0141.md) — 🟡 In progress (requested 2026-10-20; electrics, engine start, lights, doors, heating, dashboard controls; no earlier milestone owned it)
- [M-0102 — Sound Events and Force Feedback](M-0102.md) — 🟡 First slice 2026-10-20 (the player vehicle's loops and one-shots from its sound configuration; announcements, mixer inputs, force feedback open)
- [M-0103 — AI Scripts Near the Player](M-0103.md) — 🔴 Not started (proposed 2026-10-14)
- [M-0105 — Course Geometry and Path Recognition](M-0105.md) — 📝 Proposed (2026-10-14) - research only, needs approval
- [M-0053 — Articulated Vehicle Completion](M-0053.md) — ⏳ Planned (critical review 2026-10-11 F-002, F-003; scope revised 2026-10-14 from the [design](../../architecture/articulation-physics.md))

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Build the actual public transport simulation logic and the main executable.*

- [~] **🗺️ World & Map Loading** — [M-0024](M-0024.md) & [M-0025](M-0025.md)
  - [x] Document the architecture of the map streaming system (`docs/src/architecture/map-streaming.md`). *Done 2026-10-13: written from the code as it runs; limits L1-L8 and open questions Q-MS-01..05 listed.* 🔴 The code already exists without it (critical review 2026-10-11 F-012); write it before further world-loading work.
  - [~] Implement tile grid paging and async chunk streaming coordinator in `opts-game` and `opts-bevy` (M-0024 — traces to F-018). *Paging runs in `opts-game` (`stream_world_tiles`, time-budgeted resumable build since M-0089); async jobs and the `opts-bevy` module are open.*
  - [~] Implement terrain elevation mesh generation and water plane systems from `.map.terrain` and `.map.water` (M-0025 — traces to F-018). *Terrain mesh, carve, layered ground, water with depth tint and the specification ([terrain-rendering.md](../../architecture/terrain-rendering.md), 2026-10-19) done; every painted `[groundtex]` layer blended since 2026-10-25 (`9aadcf56`); open: one height service for the game, normal stitching, wave displacement.*
- [~] **🛞 Vehicle Physics & Control** — [M-0022](M-0022.md) & [M-0023](M-0023.md)
  - [x] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [x] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [x] Implement Bevy ECS bridge plugin in `opts-bevy` connecting `VehicleSimulation` state to entity `Transform` and wheel mesh rotation. (M-0023)
  - [x] Script VM (compiles all 6,062 corpus scripts; 1,216 scripted vehicles load and run) wired to the vehicle through a passive and an active Bevy bridge — OMSI Script Stack VM (`.osc`) and dynamic variable table system in `opts-sim` ([M-0026](M-0026.md) — traces to F-008, F-019).
  - [~] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators — animation, `[visible]`, interior lights, `Spot_Select` headlights, light maps, `keyboard.cfg` key triggers and `[mouseevent]` hot-spots done; vehicle night maps (2026-10-25) and `[light_enh_2]` done; in-game screenshot check and `delay` open ([M-0045](M-0045.md)).
  - [~] AI road traffic — [M-0049](M-0049.md): research and plan ([`ai-traffic.md`](../../architecture/ai-traffic.md)), AI list parsers and the `opts-traffic` crate (lane graph, car following, spawning helpers) done; traffic runs in the game (IDM lane following, speed limits and density rules, signals, conflict zones, right of way, lane changes, trailer chains); scheduled AI in M-0112, depth work in M-0111; debug overlay/`traffic.*` methods, `[splinehelper]` arms, script-driven driving and calibration open.
  - [x] Sound engine in `opts-bevy` (2026-10-20): clips, emitters, voice budget, distance fading, listener, buses with the Audio settings, `audio.*` protocol ([M-0027](M-0027.md), [audio-system](../../architecture/audio-system.md)).
  - [~] Sounds of scenery objects and splines ([M-0139](M-0139.md)), of AI traffic ([M-0140](M-0140.md)), then the player's vehicle: engine RPM and pneumatic pressure to spatial audio ([M-0102](M-0102.md)); first slices 2026-10-20, AI doors/horn/second unit 2026-10-24; announcements, `Snd_*`, force feedback, brake squeal open.
  - [x] Interior rattling ([M-0146](M-0146.md), 2026-10-26): the road textures' unevenness (`.surf` height maps, else the surface type) shakes the body on its springs, `A_Trans_Z` reaches the vehicles' rattle scripts.
- [ ] **🎛️ Vehicle Controls and Systems** — [M-0141](M-0141.md) — requested 2026-10-20
  - [~] The driver's workplace works: electrics, engine start procedure per vehicle, lights, doors, heating, wipers and every dashboard control with a function; a start-procedure test per reference vehicle (Citybus 530, MAN NL/SD200/NG272 and BRT C2 start from cold since 2026-10-20, the Luzern VBL Citybus, Valiant 7000 and MB C2 BVG families since 2026-10-25, then the TH Überlandbus, Studio Polygon Lite, Citybus S31x and others; all 948 installed buses reach drive readiness (the last, the MB O530 GDH serial hybrid, since 2026-10-25); dashboard displays: printers, VDV display and the HH109 EFAD LCD work since 2026-10-25, NG272/HH109 destination boards open).
  - [~] Articulated buses — trailing segments follow the leader kinematically, the joint angle reaches the scripts, bellows are skinned ([spec](../../architecture/articulation.md), M-0022 addendum 2026-10-11); one serialisable consist state, consist mass and β/γ in the fixed step done 2026-10-14 (M-0053 Parts 1–3), AI trailer chains done; β limits, `[couple_front]` and parked trailers open — [M-0053](M-0053.md) (critical review 2026-10-11 F-002, F-003).
- [~] **🚶 Pedestrians** — [M-0050](M-0050.md): walking pedestrians on footpath lanes with gait animation done ([design](../../simulation/pedestrians.md)); `[trafficdensity_passenger]` applied; stops and boarding through M-0101 (first slice 2026-10-21); crossings/signals, holidays, separation and LOD open.
- [~] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [x] Document the passenger simulation model in `docs/src/simulation/passengers.md` (written 2026-10-21 with M-0101) — prerequisite for M-0050's stop/boarding items (critical review 2026-10-11 F-023).
  - [~] Implement bus stop logic and passenger boarding/alighting — first slice [M-0101](M-0101.md) 2026-10-21: `[busstop]` objects, boarding through the scripts' doors, tickets, articulated buses and trams; cash desk open. *Requirement from rail operation (2026-10-12): a stop evaluates the leading head's direction of travel and door side, so a tram arriving B-head first is recognised without a mirrored second stop cube ([rail-vehicles §8–9](../../simulation/rail-vehicles.md)).*
  - [~] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`). *Vehicle routing over lanes exists (`LaneGraph::shortest_route`, used by scheduled AI and the route line on the map); passenger journey routing open.*
- [~] **🎮 The Game Binary** *(status corrected 2026-10-11, critical review F-014)*
  - [x] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App` (`crates/opts-game/src/app.rs`).
  - [~] Implement the main menu, loading screens, and basic HUD — menu (`menu/`), new-game screen, loading pipeline and in-game HUD exist; Load game opens the saves overlay (M-0069); there is no Credits screen.
- [x] **📦 Delivered outside milestones (reconciled 2026-10-10, [M-0037](M-0037.md))** — shipped after M-0023 without their own milestone; no earlier status changed.
  - [x] Workspace modularisation, `opts-catalogue` (map/vehicle catalogue) and `opts-logging` — `07de1b0`, [report](../../meta/modularization-2026-10-09.md).
  - [x] Real OMSI asset loading, tile streaming (`stream_world_tiles`), terrain mesh, data-driven sky, layered ground shader, spline attachments, camera system, humans/ego perspective — `69e94b2`, `780cae4` (crates `opts-bevy`, `opts-game`).
  - [x] Map, scenery, humans and envir parsers; `.o3d`/`.x` robustness; scrambled-mesh decoder (partial, see [M-0038](M-0038.md)) — `438d622` (`opts-core`).
  - [x] F1 reporting and GUI modal/text area/fonts — `f5dfd38`, [report](../../meta/f1-reporting-2026-10-10.md).
  - [x] Startup OMSI-installation check with folder dialog and saved path (`a68ad63`); parked vehicles from `car_park` markers and vehicle-model scenery objects (`304856e`); repaint discovery from `.bus`/`model.cfg`/`.cti` (`c00c3fc`); scrambled-mesh decoder keeps all triangles and infers UV jitter (`c00c3fc`); map-list scrolling, back buttons, absolute-height crossings (`5c752fe`).
  - [x] Control protocol and hot reload — M-0043 (see Phase 5); spline road/terrain fix — `2816806`, [bug report](../../meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md).
- [ ] **🧾 Audit-Driven Work (audit 5037f89a, 2026-10-10)**
  - [x] Reconcile roadmap, phase pages and board with work shipped since M-0023 — [M-0037](M-0037.md) (F-019, F-023).
  - [x] Complete scrambled-mesh decoding: async decoding, deterministic parameters, cache policy, generator research — [M-0038](M-0038.md) (F-001, F-003, F-014, F-017). (exact decoding 2026-10-16, `c3d10946`; heuristic decoder retired, `a17ac313`)
  - [x] Render `[texttexture]` text on signs, stops and displays — [M-0039](M-0039.md) (F-002).
  - [~] Script-driven animation, lights and doors (all but the in-game screenshot check and `delay` done) — [M-0045](M-0045.md).
  - [ ] World loader extraction — [M-0046](M-0046.md).
  - [ ] Extract the world loader into `opts-bevy` and decouple `opts-bevy` from `opts-gui` (appended to [M-0024](M-0024.md); F-015, F-016).
  - [~] Verify spline-attachment repeaters, fix floating objects, read weather/fog (appended to [M-0025](M-0025.md); F-004, F-007, F-008). — weather/fog read since 2026-10-17, repeater layout measured over the corpus; Rheinhausen floating object and an OMSI check open
- [ ] **🧾 Critical-Review Work (critical review 2026-10-11, [report](../../meta/critical-review-2026-10-11.md))**
  - [x] Host callbacks of the vehicle scripts — timetable, termini, bus stops, tickets, IBIS (`NrSpecRandom` done): destination displays and cabin monitors show their text — [M-0093](M-0093.md) (complete 2026-10-14).
  - [x] Script textures (`[scripttexture]`, `ST*` callbacks) for dash TFTs, tachographs, printers — [M-0094](M-0094.md) (complete 2026-10-14).
  - [~] Articulated vehicle completion: one articulation model with one serialised state in `opts-sim`, consist mass in the longitudinal dynamics, β/γ in the fixed step ([design](../../architecture/articulation-physics.md); scope proposed 2026-10-14), `[couple_front]`, trailers for AI and parked vehicles — [M-0053](M-0053.md) (F-002, F-003). (Parts 1–3 done 2026-10-14)
  - [ ] Move traffic, pedestrian, articulation and repaint loading out of `opts-game` with the world loader (appended to [M-0046](M-0046.md); F-009).
- [~] **🗺️ Tile Loading Controls** — [M-0080](M-0080.md)
  - [~] *Load all tiles on start* (force-loaded and kept resident), exposed in the wizard summary; *Tile radius* around the centre tile with a hardware-derived default (R-20, R-21). — both settings, live streaming and `scene.tiles` done 2026-10-18; wizard summary switch, memory guard and "Automatic (currently N)" open.
- [x] **🎥 Viewpoint Editor** — [M-0084](M-0084.md) — ✅ done 2026-10-29
  - [x] Shift + Tab (Tab until 2026-10-22) opens an editor in the driver's view (height, X/Y/Z, FOV), saved per vehicle or per model; `graphics.default_fov`; Shift + Space resets each view (R-26, R-33). Export and import of viewpoints through `camera.viewpoint.export` / `.import` (2026-10-29).
- [x] **🌀 Tile Streaming Performance** — [M-0089](M-0089.md)
  - [x] Measure where the frame time goes at tile borders, rank and implement the improvements (R-32, R-36). (complete 2026-10-18: no frame above 50 ms, worst 36.9 ms on Berlin-Spandau)
- [x] **🚌 Script rig and callback safety net** — [M-0096](M-0096.md)
- [x] **🚌 Route, timetable and IBIS data layer** — [M-0099](M-0099.md)
- [x] **🚌 Consist script state: part engines, script share and control cables** — [M-0098](M-0098.md)
- [~] **🚌 Script chain and course geometry** (proposed 2026-10-14; M-0097 implemented, M-0100/M-0101/M-0102 first slices, M-0103 started through M-0112, M-0105 still proposed) — [M-0097](M-0097.md) physics contract, [M-0100](M-0100.md) script textures, [M-0101](M-0101.md) doors, passengers, tickets and cash, [M-0102](M-0102.md) sound events and force feedback, [M-0103](M-0103.md) AI scripts near the player, [M-0105](M-0105.md) course geometry (research)
