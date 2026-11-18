# OMSI 2 versus OPTS: feature parity matrix

- Date: 2026-11-18. OPTS master 63f9b605 (read-only pass; no cargo, no game run, no repo edits).
- Inputs: OPTS docs under docs/src/ (grep and targeted reads), docs/audit/feature-audit-2026-11-18.md, docs/audit/project-audit-2026-11-18.md, ROADMAP.md and docs/src/roadmap/ milestone status lines, the OPTS crates (grep and short reads), the OMSI English manual (docs/Manual_OMSI2_EN.pdf, read as reference, no text copied) and the OMSI corpus on disk (read-only).
- Corpus counts are whole-tree grep counts over `.osc` files (about 6,200 scripts, 551 vehicle folders, 67 `cockpit.osc`), so they measure how widely a function is scripted, not how often it runs.

## Legend

Status values:
- **DONE-T**: done, with a test or a recorded live check named in the docs. Not re-run in this pass.
- **DONE-U**: done in code, no test or live check named, or the docs say untested.
- **PARTLY**: a first slice or script-driven path exists, with a stated open item.
- **MISSING**: no code path found by grep, and no milestone in progress.
- **PLANNED**: a milestone is proposed, specified or deferred, and no code exists.
- **N/E**: not established in this pass. Do not treat as present or absent.
- **OOS?**: no decision recorded. Could be deliberate. Flagged so the owner can decide.

Size (for gaps): S = a few files and tests, one slice. M = several slices or one new integration. L = a new subsystem or data path.

Source tags: `manual §x.y` = OMSI English manual section; `corpus:` = path under the OMSI install; `cat §` = docs/src/simulation/vehicle-functions.md (dated 2026-10-16, so treated as possibly stale); `MNNNN` = roadmap milestone.

## Summary

OPTS covers the core of OMSI's driving model well: file formats, the vehicle script VM, the drivetrain and brakes, the road and tram control scheme, IBIS and timetable data, lights, weather presets, the map view, session vehicles and save/restore, and scheduled AI traffic. OMSI's bus-operation layer (stops, passengers, tickets, timetable discipline, breakdowns, service) is where OPTS is thinnest: a first passenger slice exists, cash and change do not, and there is no failure or repair system.

Four audit findings still bear on parity and are confirmed in this pass: the default free drive takes no passengers, tunnel zones are never built in play, the trailer-end seam is unverified across angles, and the battery-off path has no test. The Help table is still missing several live keys. The bus-operation and failure layers are the largest gap by player-visible impact.

## Area tables

### A. Script VM and formats

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Vehicle `.bus`/`.osc`/varlist/const parsing | Full script sets per bus; about 6,200 `.osc` in corpus (corpus:Vehicles/*/Script/) | DONE-T | M-0014, M-0026 (complete for vehicles), M-0096 |
| Callback and event model | `callbacks.md` names; events drive sound and state | PARTLY | cat § "Events"; events raised for sound are deferred (P6), `E7` engine-raised triggers deferred |
| Map, scenery and texture formats (`.sco`, `.sli`, `.tga`, `.dds`, `.o3d`, `.x`) | corpus:Sceneryobjects, corpus:maps | DONE-T | M-0013, M-0015, M-0018, M-0019 |
| Script textures (`[scripttexture]`, `ST*`) | Destination displays, canvases | PARTLY | M-0094 done; M-0100 partly built (fonts and some `ST*` calls open) |
| Text textures (`[texttexture]`) | Static signs and displays | DONE-T | M-0039 (static strings; named sources open) |
| Setvars per bus | manual §2 options; corpus `setvar` | DONE-T | M-0078, M-0110 |

### B. Drivetrain and brakes

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Engine start and stop, cold start | manual §4.8.1 (key, battery, charging light, oil pressure, air pressure, neutral); corpus `engine_GN.osc` | DONE-U | ROADMAP Phase 4 line: all 948 buses reach drive readiness (since 2026-10-25). Start procedure runs through scripts |
| Engine model (torque, idle, temperature, consumption) | cat § "Engine model" | DONE-U | opts-sim vehicle; M-0026 |
| Automatic gearbox, converter, kickdown | cat § drivetrain; manual §4.8.1 gears 3/D | PARTLY | M-0097 physics contract "Implemented" with open items; drivetrain in opts-sim |
| Retarder | corpus: 924 scripts mention retarder | DONE-U | opts-sim `vehicle/drivetrain/step.rs`, `vehicle/inputs.rs` |
| Gear selector and start interlocks | cat § "Over-rev and start interlocks" | PARTLY | D/N/R keys in road scheme (vehicle-controls.md); interlocks script-driven, not tested in this pass |
| Service brake with air build-up | manual §4.8.1; cat § | DONE-T | M-0073 (per-axle brakes with pneumatic build-up, Complete 2026-10-18) |
| Parking brake (`.` key in road scheme) | manual §4.8.1; cat § | DONE-U | vehicle-controls.md line 10 (current key is `.`) |
| Stop brake (Haltestellenbremse) | manual §4.8.2; cat § | PARTLY | Script-driven through the physics contract (M-0097); no separate test found |
| ABS and ASR | cat § "ABS and ASR" | N/E | opts-sim has wheel slip code (`vehicle/brakes.rs`, `wheels.rs`), but no ABS or ASR logic was found by grep |
| Tram rail brake, sanding, PZB | cat § "Tram brakes" (no signalling input) | MISSING | Catalogue marks PZB and rail brake deferred |

### C. Cab controls, displays and IBIS

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Gauges and warning lamps | cat § gauges, "inputs (done)" | DONE-U | cat § coverage table |
| Interior, dashboard, reading and headlights | manual §4.7; corpus `cockpit.osc` | DONE-U | M-0045 (lights done; in-game screenshot check open) |
| Brake and reversing lights | cat § lighting | DONE-U | M-0045 |
| Indicators and hazard | manual §4.7 (18 hits); corpus 1,048 scripts mention indicator/blinker | PARTLY | Scripted; no indicator or blinker tests found (feature audit FEAT-12) |
| Bulb failures | cat § "Bulb failures" (rare) | MISSING | Needs `E7` engine-raised triggers, deferred |
| IBIS line, route and course | manual §4.2.1; cat § IBIS | DONE-T | M-0099 Done 2026-10-19 (protocol 1.13.0); M-0093 (host callbacks, IBIS) Complete |
| Current and next stop, stop list | cat § IBIS | PARTLY | hof-based (M-0093); `act_busstop` read-back and `Refresh_Strings` marked deferred in cat § |
| Destination display (matrix, roller blind, plug-in sign) | manual §4.2.1 and §6.3; cat § | PARTLY | M-0100 partly built; M-0039 static text done |
| Status display (Shift+Z, red lines) | manual §3.2 | N/E | Not located in OPTS docs or grep |
| Horn and bell | cat § horn; corpus horn used in 30 OPTS source files | PARTLY | Sound events M-0102 first slice (2026-10-20) |
| Force feedback | cat § "Force feedback" (no milestone) | PLANNED | M-0102 partial; no milestone for FF beyond that |

### D. Doors, stops and passengers

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Door open and close with interlocks | manual §4.8.2; corpus about 2,300 scripts mention doors; `Pneumatisch` appears 719 times | PARTLY | Door state read from `pax_*_open` and `PAX_Entry<n>_Open` script variables (`opts-game/src/passengers/system/doors.rs:6-17`). Arming logic lives in scripts, not Rust (feature audit FEAT-03) |
| Boarding only for expected destinations | manual §4.2.1: passengers board only buses showing a destination they expect | PARTLY | Boards when the trip's destination pair matches (`opts-sim/src/passengers/destination.rs:88`). A free drive has no trip, so nobody boards (FEAT-05) |
| Stop request light and passenger stop button | manual §4.2.2; cat § "stop request" (deferred P5) | MISSING | No stop-request handling in opts-game (grep hits only in capture code). Passengers doc says alighting follows ride length, not real stops |
| Rear doors, baby-carriage switch, stop-brake interlock | manual §4.8.2 items 2 and 5–6 | N/E | Script-driven. Door camera tests mention rear doors (`opts-bevy/src/door_camera/tests`), not logic |
| Wheelchair ramp and kneeling | cat § "Ramp and kneeling"; corpus `Rampe` 530 scripts, `Kneel` 681 | PARTLY | Kneel affects suspension (`opts-sim/src/vehicle/suspension/*`); ramp behaviour not confirmed |
| Passenger count, seats, places, alighting | manual §4.2.2 | PARTLY | M-0101 first slice (2026-10-21); passengers.md §2–§3 |
| Greetings, complaints, passenger voices | manual §4.2.2 | MISSING | passengers.md §4 "Voices: not played" |
| Crowds and people colliding | not stated in manual | PARTLY | passengers.md §4: no collision between people |
| Pedestrians (walkers) | corpus `humans.txt`; OMSI walkers | PARTLY | M-0050 walking done; crossings and stops open |

### E. Ticketing and money

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Ticket packs and prices | manual §6.1; corpus `TicketPacks` | DONE-U | passengers.md §2 (`GetTicketName`, `GetTicketValue`); 64 packs parsed (tests) |
| Ticket decisions by passenger | manual §4.2.3 | DONE-T | passengers.md §3 "Verified" (unit tests on tickets) |
| Ticket printer (advanced mode) | manual §4.2.3 | PARTLY | Coverage table: printing "done (tickets)"; driver-side printer interaction not confirmed |
| Cash desk and change (Shift+T, T, Ctrl+T) | manual §4.2.3; `GiveChangeCoin` in corpus | MISSING | `GiveChangeCoin` is logged only; cash desk open (passengers.md §2, §4) |
| Fare computation and stop-dependent prices | manual §4.2.3 (prices rise 1986–94) | MISSING | cat § "Fare computation" (rare, deferred) |

### F. Timetable, schedule and traffic signals

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Route, timetable and circuit data | manual §4.2.4; corpus `maps/*/TTData/*.ttl` | DONE-T | M-0099 Done 2026-10-19 |
| Circuits selectable by day type and time | manual §4.2.4 (circuit by weekday; night lines 5N/13N only at night) | N/E | No circuit selection in the docs; passengers.md §4 says day types are not applied to passenger weights |
| Take over or hand over the AI bus | manual §3.3.5–3.3.6, §4.2.4 | PARTLY | `opts-bevy/src/cab_switch/handover.rs` exists; return to depot on a maintenance circuit not confirmed |
| Additional bus alongside the AI bus | manual §4.2.4 | N/E | Not located |
| Scheduled AI buses, trams, trains | (OPTS feature; OMSI has AI on schedules) | DONE-T | M-0112 Done 2026-10-23 |
| Special bus-priority signals (Spandau) | manual §6.5: bus registers with signal; stop, go and transition | N/E | M-0111 traffic lights in progress; bus-priority behaviour not found |
| Track switches | OMSI points behaviour not established in this pass (OPTS feature) | DONE-T | M-0113 Done 2026-10-30 |

### G. Lighting, weather and seasons

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Weather presets (9 `Weather/*.owt`: Sommerlich, Starker Schneefall, Bodennebel, Sturm, and others) | manual §2.5; corpus `Weather/` | DONE-U | docs/src/architecture/weather.md reads OMSI's own presets (implemented 2026-10-17) |
| Rain, dirt and spray, wipers and washers | cat § wipers and rain; corpus wiper 1,400 scripts | PARTLY | Wiper scripts run from `PrecipRate` (23 OPTS source files); on-screen check not in docs |
| Fog and defrost | manual "fog" lights; corpus fog 428 scripts | PARTLY | Fog in weather effects; defrost not established (cat § "Fogging and defrost" rare) |
| Snow | corpus `Event_Schnee.osc`, `Schnee.osc`; no manual section | PARTLY | Snow flag loads after textures (FEAT-09); M-0121 in progress |
| Wetness and puddles | cat-independent; corpus `[moisture]`, `[puddles]` | PARTLY | M-0121 in progress (texture flags parsed, wetness ramp) |
| Night lights and night maps | corpus `night` 98 scripts; `[matl_nightmap]` | DONE-U | M-0045 (`[matl_nightmap]` 2026-10-25) |
| Heater and HVAC | manual §4.8.1 (auxiliary heater); corpus heater 705 scripts | PARTLY | Heater state via scripts; `Cabinair_Temp` consumer not confirmed (cat § HVAC) |
| Tunnels and covered sections | corpus `ABANZ.osc`, `NF6D_A_Kopf.osc` (26 scripts mention tunnel) | MISSING (in play) | Detector exists but tile loader passes no splines: `opts-game/src/in_game/omsi_world/tile/zones.rs:29`. M-0115 in progress |
| Real weather from observations | Not in OMSI (manual has no real-weather feed) | PLANNED | M-0116 research only, pending review. Owner-directed, not OMSI parity |

### H. Damage, failures and service

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Collisions with scenery and vehicles | manual §2.11, §3.3.7 | PARTLY | collisions.md "First version": static objects, AI vehicles, ground steps. Trailers pushed by head only (collisions.md item 4) |
| Failures and malfunctions | corpus `elec_failure_general` (1,584), `antrieb_failure_general` (1,095), `engine_failure_general` (563), `door_back_failure` (219) | MISSING | No failure-injection code found by grep. Catalogue marks engine-raised triggers deferred (`E7`) |
| Maintenance status (damage from lack of maintenance) | manual §3.3.7 | MISSING | Option not found in docs |
| Repair | manual §3.3.7 | MISSING | No repair action; `repair` matches texture code only |
| Wash at depot or station | manual §3.3.8 | MISSING | `veh_wash` trigger unbound in corpus scan (M-0120 notes) |
| Tank up at depot or station | manual §3.3.9 | PARTLY | Refuel key F12 works anywhere (fuel-battery.md §5); stop-based tanking and its time not modelled |

### I. Electrics, battery and fuel

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Main battery switch and charge light | manual §4.8.1; corpus `Batterie` 1,285 scripts | PARTLY | Battery toggle trigger `cp_batterietrennschalter_toggle` (`opts-bevy/src/vehicle_setup/steps.rs:93`). Battery-off path has no test (FEAT-08) |
| Fuel level and consumption | manual §3.3.9; corpus `Tank` 1,103 scripts | PARTLY | M-0123 in progress: model, setting, HUD (on-screen check open), `tank_percent` input; fuel-battery.md §5 |
| Electric bus state of charge | corpus electric buses (HEULIEZ-GXx37-Elektro, HH_EBus) | N/E | fuel-battery.md: the variable carrying state of charge is not in the predefined list |

### J. Vehicles, sessions and saves

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Multiple buses; switch, add, delete | manual §3.3.1–3.3.3 | DONE-U | M-0068 Implemented and verified live (2026-10-13), session vehicles |
| Place a bus at a map point or preset | manual §3.3.4 | N/E | Not located |
| Save and load a situation | manual §3.1 | DONE-T | M-0069 Done 2026-10-13 (session save and restore) |
| Bus configuration presets | corpus `option_presets/` | DONE-T | M-0077 Complete 2026-10-30 |
| Livery and repaint selection | manual §1.2 (customisation); corpus repaints | PARTLY | M-0032 selection and load-time application shipped; editing not in scope |

### K. Map, world, traffic and pedestrians

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Map view and game menu | manual §2.5 | DONE-U | M-0108 (full-size map, F6) Done 2026-10-19 |
| Tile streaming | OMSI streaming behaviour not established in this pass | DONE-U | M-0024 partly delivered; M-0089 Complete; M-0114 hybrid streaming in progress. OPTS goes beyond OMSI here |
| Terrain and water | corpus `.water` tiles | PARTLY | M-0025 partly delivered; M-0067 in progress |
| AI road traffic | corpus traffic scripts; manual §6.5 | PARTLY | M-0049 traffic runs in game; M-0111 in progress (obstacles, lane changes, lights) |
| Parked cars, buildings and mesh fidelity | corpus scenery meshes | PARTLY | M-0066 in progress; M-0090 in progress |
| Level crossings | M-0111 mentions barrier handling; corpus not checked for OMSI crossing scripts | PARTLY | M-0111 barrier handling in progress |

### L. Trams, rail and trolleybuses

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Tram traction and consist (NF6D, three cars) | corpus `Vehicles/Tram_NF6D` | PARTLY | M-0098 Done 2026-10-21 (per-part engines and consist); M-0106 formation editor Done |
| Track network and AI trains | OMSI tram timetables in corpus (`maps/Ruhrgebiet/TTData/302_Tram.ttl`, `maps/Tram/TTData/107_TRAM.ttl`) | PARTLY | rail-vehicles.md §7 "first cut" (R1–R3) |
| Rail control scheme (lever, direction) | vehicle-controls.md | MISSING | "rail scheme (lever, direction) not yet" (vehicle-controls.md line 5) |
| Pantograph and overhead line | cat § "Pantograph" (no overhead-line input) | MISSING | No `pantograph` hits in crates |
| Trolleybus (overhead wire) | not established in this pass | N/E | Electric buses found in corpus; overhead-wire use not checked |

### M. Sound, input devices and camera

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Engine, scenery and AI sound | corpus `Sounds` (events) | PARTLY | M-0102, M-0139, M-0140 first slices (2026-10-20) |
| Interior rattles and road shake | OPTS addition; OMSI rattle behaviour not checked | DONE-U | M-0146 Implemented 2026-10-26 |
| Joystick and wheel | manual §2.2 | PARTLY | Gamepad and wheel rebinding open (M-0142, phase 7) |
| TrackIR head tracking | manual §4.1 | MISSING (OOS?) | No TrackIR code or doc hit in crates or docs |
| Driver, passenger, external and map views | manual §2.7 (F2–F4) | DONE-U | camera-system.md; M-0109 door camera Done 2026-10-29 |

### N. Settings, help and UI

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Keyboard reference | manual §1.8 | PARTLY | Help table incomplete (see GAP-18) |
| Settings registry | manual §2.1 options | PARTLY | M-0063 started; 22 settings and protocol done |
| Accessibility and localisation | (OPTS addition) | PARTLY | M-0065 automatic part done; M-0083 language packs implemented |
| UI scale and font settings | (OPTS addition) | DONE-U | M-0109 Done 2026-10-29 |
| Start time from the map | manual §2.8 (set time) | DONE-T | M-0070 Complete 2026-10-18 |

### O. Editors and content tools

| Feature | OMSI behaviour (evidence) | OPTS status | OPTS evidence |
|---|---|---|---|
| Route editor | manual (route editor named in §1.1) | PLANNED | M-0051 editor mode specified; M-0055, M-0056, M-0092 specified |
| Vehicle mesh and profile editor | (OPTS plan) | PLANNED | M-0135 to M-0137 proposed |
| Viewpoint editor | (OPTS feature) | DONE-U | M-0084 Done 2026-10-29 (OPTS ahead of OMSI here) |
| Train formation editor | (OPTS feature) | DONE-T | M-0106 Done 2026-10-15 |

## Ranked gaps (top 20)

Ranking is by player impact on driving feel, the world and the simulation, with the largest gaps first. Niche and editor-only items are ranked last.

| ID | Area | OMSI evidence | OPTS status | Size | Milestone | Dependency |
|---|---|---|---|---|---|---|
| GAP-01 | Passengers | manual §4.2.1: passengers board only expected destinations, so a bus needs a display setting. Free drive is the default | MISSING for free drive (`admits()` needs both destinations; `destination.rs:88`) | S | M-0101 (done, no caveat) | Decide the rule: free drive picks the stop's line, or a default trip |
| GAP-02 | Articulation | corpus articulated buses (NG272, GN92) | PARTLY: trailer-end seam unverified across angles and speeds; pitch and roll open | M | M-0053 | Multi-angle live test (feature audit FEAT-01, FEAT-02) |
| GAP-03 | Damage and failures | corpus failure variables (over 3,000 uses); manual §3.3.7 breakdowns | MISSING: no failure injection, repair or maintenance option | L | none (catalogue `E7` deferred) | Engine-raised triggers; event system research (M-0118) |
| GAP-04 | Tunnels and zones | corpus tunnel scripts (`ABANZ.osc`, `NF6D_A_Kopf.osc`) | PARTLY: detector built, tile loader passes no splines (`zones.rs:29`) | M | M-0115 | Feed parsed splines; label one real tunnel |
| GAP-05 | Electrics | manual §4.8.1 battery switch; corpus `Batterie` 1,285 scripts | PARTLY: battery-off path untested; trigger name differs from M-0123 text | S | M-0123 | Script-rig test; check four `cockpit.osc` files (FEAT-08) |
| GAP-06 | Ticketing | manual §4.2.3 cash desk, change keys | MISSING: cash desk and change not built (`GiveChangeCoin` logged only) | M | M-0101 (cash desk open) | Money model; ticket printer UI |
| GAP-07 | Timetable | manual §4.2.4 circuits by weekday and night | N/E: no circuit or day-type selection found | M | none (M-0099 and M-0112 done without it) | Calendar and circuit data in TTData |
| GAP-08 | Stops | manual §4.2.2 stop request light; §4.8.2 rear doors and baby switch | MISSING for stop request; rear doors script-driven only | S | M-0101 (partly) | Door and stop scripts; passenger stop state |
| GAP-09 | Rail controls | SUSPECTED: tram scripts read direction and lever (not read in this pass; corpus `Vehicles/Tram_NF6D`) | MISSING: rail lever and direction scheme not built | M | M-0141 (first slice, road only) | Map rail direction and lever to tram script variables |
| GAP-10 | Trolleybus and pantograph | cat § "Pantograph" (no overhead-line input) | MISSING | L | none | Overhead-line input; consist power |
| GAP-11 | Traffic signals | manual §6.5 bus-priority signals (Spandau) | N/E: signal phases in M-0111, bus priority not found | M | M-0111 (in progress) | AI signal model |
| GAP-12 | Pedestrians | manual §4.8.2 and corpus crossings | PARTLY: no signal phases, no yield to traffic | M | M-0050 | Needs M-0111 signal model |
| GAP-13 | AI traffic | collisions.md item 1 (AI should see player); corpus traffic | PARTLY: AI does not react to the player | M | M-0111 (in progress) | Traffic crate neighbour query |
| GAP-14 | Passengers | passengers.md §4 real next stop alighting | PARTLY: ride counts stops reached, not timetable stop sequence | M | M-0101 | `bus.set_stop` coupling |
| GAP-15 | Snow and wetness | corpus `Event_Schnee.osc`; `Starker Schneefall` preset | PARTLY: snow flag read only at texture load (FEAT-09) | S | M-0121 (in progress) | Refresh loaded textures on flag change |
| GAP-16 | Sound | manual §4.2.2 passenger greetings; corpus `TicketPacks/*/<voice>/` | MISSING: voices not played | M | M-0102 / M-0140 (partly) | Audio payload events (P6) |
| GAP-17 | Cabin climate | manual §4.8.1 auxiliary heater; corpus heater 705 scripts; cat § HVAC | N/E: heater and HVAC not confirmed on screen | S | none | Environment inputs; `Cabinair_Temp` consumer |
| GAP-18 | Help and keys | manual §1.8 keyboard reference | PARTLY: F3, F11, Shift+F7, Ctrl+F7, Shift+F8 not listed in help/controls (F12 refuel row added) | S | M-0064 / M-0142 | Reverse test that every bound key is listed (FEAT-04) |
| GAP-19 | Service | manual §3.3.8 wash; §3.3.9 tank at depot | MISSING wash; tanking partly (F12 only) | M | M-0123 (tank), none (wash) | Depot zones (`opts-bevy/src/route/depot.rs`) |
| GAP-20 | Editors | manual route editor and livery customisation | PLANNED only | L | M-0051, M-0032 | Editor mode design; OPTS prioritises driving first |

Not in top 20, deliberately: TrackIR (MISSING, OOS? with no decision); status display Shift+Z (N/E); ABS and ASR (N/E); stop-brake and parking-brake details (script-driven); bulb failures (MISSING, E7); tram PZB and sanding (MISSING, no signalling input).

## Notes on the top gaps

- **GAP-01 is the most player-visible.** The default drive takes nobody on. OMSI gates boarding on the destination display (manual §4.2.1), so in OMSI the player sets the display before passengers board. OPTS's free drive has no display setting that reaches the boarding rule. The fix may be a stop-based default, or a destination the player can pick, rather than a full timetable.
- **GAP-03 is the largest.** OMSI's corpus scripts carry about 3,000 failure uses. OPTS has no failure injection. Owner decision needed on whether breakdowns are wanted at all, since they shape the driving loop.
- **GAP-02 and GAP-04 are partly done.** The code exists, but the docs and board overstate them (feature audit FEAT-01, FEAT-06).
- **GAP-05 is cheap to verify.** A script-rig test would confirm or refute the battery-off path.
- **GAP-18 is cheap and useful.** A reverse test of the Help table would have caught the missing keys.

## Discrepancies found while building the matrix

- Feature audit FEAT-04: F12 refuel row now present (`help/controls/driving.rs:33`). F3, F11, Shift+F7, Ctrl+F7 and Shift+F8 are still not listed (grep of `panels.rs` and `driving.rs`).
- Feature audit FEAT-03: the passengers doc says the scripts write `PAX_Entry<n>_Open`, so arming is script-side. This does not contradict the audit, but the claim "pneumatic arming" in the brief was not located in Rust.
- `docs/src/simulation/vehicle-functions.md` coverage table is dated 2026-10-16. Several rows (IBIS, lights, ticket printing) have since moved, so the table was used only as a list of functions, not as status.
- Milestone M-0115 status (2026-11-17) confirms the tile loader still passes no splines.

## What I could not establish

- Whether OMSI's door arming, stop-request and rear-door logic matches OPTS's behaviour. These are in scripts, and only grep counts were taken.
- The OMSI status display contents (manual §3.2 describes it; the corpus was not searched for its variables).
- Whether OMSI's bus-priority signals are coded in scripts or in map data. Only the manual's diagram was read.
- Whether OPTS's wiper, fog, heater and indicator scripts produce the on-screen result. No game run was done.
- Trolleybus behaviour in OMSI (electric buses exist in the corpus; overhead use not checked).
- OMSI maintenance-status option values and failure rates (the options dialogue is not in the corpus).
- Exact test results. No test was run, so DONE-T means the docs name a test or live check, not that it passes today.
- The OMSI addon PDFs (for example the Perlschnur timetables) were not read.

## Sources used

- OPTS: docs/audit/feature-audit-2026-11-18.md, docs/audit/project-audit-2026-11-18.md, ROADMAP.md (phase table), docs/src/roadmap/phase-*/M-*.md (status lines), docs/src/simulation/{passengers,fuel-battery,collisions,pedestrians,rail-vehicles,vehicle-controls,vehicle-dynamics,vehicle-functions,vehicle-keys}.md, docs/src/architecture/weather.md, crates/opts-game/src/passengers/system/doors.rs, crates/opts-game/src/in_game/omsi_world/tile/zones.rs (line 29), crates/opts-sim/src/passengers/destination.rs (line 88), crates/opts-bevy/src/vehicle_setup/steps.rs (line 93), crates/opts-game/src/help/controls/{driving,panels}.rs.
- OMSI reference: docs/Manual_OMSI2_EN.pdf (text extracted to scratch for reading; sections 2.5, 2.7, 2.8, 2.11, 3.1–3.3, 4.1, 4.2, 4.7, 4.8, 6.5 read).
- OMSI corpus (grep counts and file paths only): Weather/*.owt, Vehicles/Tram_NF6D/script/, Sceneryobjects/HB_76_Objekte/Kleine Objekte/Script/Event_Schnee.osc, Sceneryobjects/*/ABANZ.osc, maps/*/TTData/*.ttl.
