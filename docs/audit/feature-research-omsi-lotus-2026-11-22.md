# OMSI 2 and LOTUS Simulator feature research against OPTS

- Date: 2026-11-22.
- OPTS reference: master at f86f2d5b (the 2026-11-18 parity matrix cites 63f9b605; the differences that matter are noted in section 5).
- Inputs: two research inventories kept outside the repository (48 OMSI 2 features, 49 LOTUS Simulator features), the existing audit set under `docs/audit/`, ROADMAP.md and the roadmap pages.
- Scope: documentation only. No build, no game run, no code change.

## 1. Executive summary

This report compares OPTS with two bus and tram simulators. OMSI 2 is the reference for parity, because OPTS reads OMSI 2 content and runs its vehicle scripts. LOTUS Simulator is a second reference, used to find features that a player of a modern bus simulator might expect and that OMSI 2 does not show. The two inventories hold 97 feature entries in total. Each entry was mapped to one of five outcomes: done, open with an existing GAP or ISS id, open with no id (a new RG item), out of scope, or unresolved because the sources conflict or are silent.

The main result is that the existing register already describes most of the open gaps. Of the 97 entries, 31 map to an open gap that GAP-01 to GAP-20 or ISS-001 to ISS-041 already records. 27 are done according to the evidence. 23 fall under the new RG items, and 11 are out of scope for a clean-room project or are paid add-on content. Five are unresolved. Twelve RG items are proposed, one of them major and the rest minor. Three of those twelve come from the 2026-11-18 matrix rather than from the two inventories.

The five largest gaps for OPTS, in order:

1. **Career and progression (RG-06, major).** The vision commits to a single-player career with progression. No milestone or roadmap line plans it. OMSI 2's career features are in a paid add-on, not the base game, so this is a vision gap rather than an OMSI parity gap. It is still the largest item a player would expect and no document owns it.
2. **ABS and ASR driver aids (RG-02, minor, provisional).** The brake model states that it has no ABS. Whether the OMSI vehicle scripts supply these aids was not checked. If they do not, the gap becomes major.
3. **Collision response beyond the first version (RG-03, minor).** Both inventories mention collisions. OPTS has a first version covering static objects, AI vehicles and ground steps, and the trailer case is only partly covered.
4. **Stop-dependent fares and fare computation (RG-12, minor).** The cash desk and change are already covered by GAP-06 (milestone M-0156). Fare prices that depend on the stop, and the price rises across the chronology years, are not in any GAP or ISS entry.
5. **TrackIR head tracking (RG-04, minor).** OMSI 2 lists it as a verified base feature. OPTS has no code, and the 2026-11-18 matrix left it as out of scope with no owner decision.

The largest player-visible gaps were already in the register and remain open. Failures and repair (GAP-03), the cash desk and change (GAP-06) and stop requests (GAP-08) are the three with the most impact. One register entry is stale. GAP-01, the free-drive boarding rule, was listed as missing in the 2026-11-18 matrix. The rule and its boarding-path tests are on master now. The in-game acceptance check is still open under M-0152, so GAP-01 is partly closed and should not be read as missing.

What is not known is listed in section 7. The important unknowns are: whether the OMSI vehicle scripts already supply ABS, ASR and the electric state of charge; how the chronology date is chosen in the menu; what the LOTUS Rails module contains; and the contested LOTUS facts about weather, the Berlin map and the engine lineage. No LOTUS status that the sources disagree on has been resolved.

## 2. Method and limits

### 2.1 Sources and how they were weighted

The OMSI 2 inventory rests on the publisher's store page and the Aerosoft product page, which count as primary but are marketing copy. Community sources (the Steam discussions, a forum thread, a fan wiki, reviews and press) were used for the passenger, ticket and door detail, because the store page lists none of those features. Several community items were read only as search snippets. The inventory tags each entry as verified, community or unverified, and this report keeps those tags unchanged. Entries tagged verified mean "stated by the publisher", not "checked in game".

The LOTUS inventory rests on the Steam store pages for the base game and the Streets module. Reviewer and aggregator summaries (a review aggregator, a score site, a forum) add community context. Several LOTUS facts come from a summarising model's extraction of a page, not from the raw page text, and the inventory notes this. The tags are V (verified, from the vendor's own page), C (community) and U (unverified or sources conflict). They are kept as given.

### 2.2 Weaknesses that matter for the conclusions

- **OMSI base manual not read in this inventory.** The game ships a manual in its docs folder. The research notes say a user reports that it covers buses, functions and timetables, but this pass did not read it. The 2026-11-18 matrix cites an English manual at `docs/Manual_OMSI2_EN.pdf`. That file is not present in this worktree, so the manual-based statements in the matrix could not be re-checked here. Door, ticket and timetable entries that rely on community sources would be upgraded or corrected by the manual.
- **OMSI patch state unknown.** The base game was released in 2013. The base-game patch record found was a SteamDB update date with no changes listed. The add-on changelogs found do not describe the base game. The features listed may therefore differ from the current build.
- **LOTUS access limits.** Indie DB, the SteamDB patch notes and the community changelog could not be read. No official website, manual or patch history was found. The developer's last stated update is reported as more than four years old on the store page, while a third-party listing shows an update dated January 2026. The inventory cannot resolve this.
- **Paid add-ons.** The OMSI career and economy features sit in a paid add-on, Bus Company Simulator, and the timetable generator and ticket editor sit in a paid Power Toolkit. This report uses only their store-listing descriptions to classify them. No add-on content, PDF or handbook was read for this report.

### 2.3 The contested LOTUS facts

These items are unresolved and this report does not decide them.

- **Engine lineage.** Forum opinion is split on whether LOTUS shares code with OMSI 2. One player says the developers rewrote the engine. No source found settles this, and it has no bearing on OPTS's clean-room method. It is not used as a basis for any classification here.
- **Weather and seasons.** The store page describes realistic, unpredictable weather and seasons. The Early Access plan lists them as not yet implemented, and reviewers say weather is absent. Both are tagged U in the inventory and this report keeps that.
- **Berlin map.** One store snippet lists a Berlin map as implemented, and the full page says it is not yet implemented. The inventory records the claim as community-level only.
- **Passenger simulation and AI traffic.** The store page lists both as implemented. Reviewers call both half-finished. The inventory tags passenger simulation U (sources conflict) and the AI traffic claim V (store claim only). This report treats both as open until the reviewers' point is checked in play.

### 2.4 OPTS-side method

OPTS status comes from two sources. The first is the 2026-11-18 parity matrix and issue register, which were built from grep and short reads on master 63f9b605. The second is this pass's targeted checks on f86f2d5b. The checks counted files that mention each feature term across `crates/` and `docs/src/`, and read the specific lines behind the claims that matter. The main checks are listed here.

- Free-drive boarding: `crates/opts-sim/src/passengers/destination.rs:91-98`, the `admits` function and its free-drive arm; commit d834b520 (boarding-path tests) and commit 7e465ce7 (the rule), both ancestors of master.
- ABS: `crates/opts-sim/src/vehicle/brakes.rs:8`, which states the brake model has no ABS.
- Chronology runtime: `crates/opts-game/src/in_game/omsi_world/chrono.rs`, the module header, which describes overlays applied for the session's map date and a live switch on a date change.
- Validator: `crates/opts-sim/src/passengers/state.rs:25` and `spec.rs:37`, which model stamping at validators.
- Editor scope: `docs/src/editor/README.md`, which lists timetables, terrain, splines and traffic rules as planned editor phases.
- TrackIR, VR, tutorial, career: no code hits for TrackIR or a tutorial, and no VR code. The `vr` pattern matched two documents that do not appear to concern VR. Career appears in the vision, two meta review pages and a phase 9 flow page, and no milestone plans it.
- Rebinding: `crates/opts-game/src/vehicle_keys.rs` and the help modules. The milestone record (M-0142) says the first slice is done and gamepad and wheel rebinding are open.

No test was run in this pass, and no game was run. Where the matrix marks an item DONE-T, that means its docs name a test or a live check. It does not mean the test passes today.

## 3. Feature matrix

The matrix has one row per feature area. Each cell names the feature IDs from the two inventories (OMSI 2 D1 to K3, and LOTUS numbers 1 to 49). The OPTS column uses three words: **done**, **partial** and **not started**, with the evidence and the milestone. The gap class is one of four labels: **parity gap**, **differentiator** (a feature neither reference the project aims at has, or a vision item), **out of scope** (clean-room or paid add-on content, or a non-feature criticism), or **done** where no gap remains.

| Area | OMSI 2 | LOTUS | OPTS today | Gap class | Register id |
|---|---|---|---|---|---|
| Driving model and physics | D1 (community, praised), D3 (verified, script engine) | 1 (V) | Done with a caveat. Vehicle physics in `opts-sim`; script VM tested (matrix A). Driving feel still has open issues | Done | ISS-040, ISS-041 |
| Articulated buses and trailers | D2 (verified) | 4 (V, content) | Partial. The trailer-front seam is unverified across angles, pitch and roll | Parity gap | GAP-02, ISS-011, ISS-037 |
| Vehicle lineup and custom vehicles | V1, V2, V3 (verified) | 4, 5, 7, 8 (V, content) | Done for installed OMSI content. Plays the vehicles in the installed OMSI folder. LOTUS vehicles are not used (ADR-0002) | Done; LOTUS content out of scope | none |
| Trams, trains and trolleybuses | none in OMSI inventory | 3 (V), 6 (U, Rails module contents unknown), 9 (V, train assembly) | Partial. Per-part tram consist and formation editor done (M-0098, M-0106). AI trams and trains done (M-0112). Rail lever and direction not built (GAP-09). Trolleybus and pantograph not started (GAP-10) | Parity gap | GAP-09, GAP-10 |
| Vehicle component swaps | none | 11 (V: displays, on-board computers, ticket printers) | Not started. Vehicle profile overrides are proposed in Phase 14 (M-0135) | Differentiator | RG-11 |
| Doors | P1 (community, per-door bindings) | 14 (U) | Partial. Door state is read from script variables; arming logic is in scripts and its tests are parse-only | Parity gap | ISS-013 |
| Stop requests and extra doors | C3 (community, third-door switch on the stop brake) | none | Not started for stop requests. Rear and stop-brake door logic runs through scripts, unverified | Parity gap | GAP-08 |
| Boarding rule | P2 (community, timetable and route indicator) | 13 (U, passenger simulation half-finished) | Partial. Free drive now boards anyone whose destination the stop serves (`destination.rs:94-95`, tests in d834b520). In-game check open under M-0152 | Parity gap, partly closed | GAP-01, ISS-036, GAP-14 |
| Boarding voices and announcements | P4 (community, boarding announcement) | none | Not started. Passenger voices are not played (passengers doc section 4) | Parity gap | GAP-16 |
| Passenger state and save recovery | P5, S1 (community) | none | Done. Session save and restore (M-0069) | Done | none |
| Passenger views and cameras | P3, C4 (verified) | none | Done with a caveat. Vehicle view modes and the door camera (M-0109) | Done | none |
| Fare types and ticket packs | T1 (community) | none | Done for ticket packs and decisions (passengers doc sections 2 and 3). Fares that depend on the stop are not built | Done for packs; fares see RG-12 | RG-12 |
| Ticket validator, printer and change | T2, T3 (community) | 15, 16 (U) | Partial. Validator stamping is modelled in `opts-sim` (`passengers/state.rs:25`). Cash desk and change not built; `GiveChangeCoin` only logged | Parity gap | GAP-06 (milestone M-0156) |
| Ticket pack editor (paid tool) | T5 (verified, paid add-on) | none | Out of scope. Paid add-on tool | Out of scope | none |
| Career and money | T4 (unverified; paid add-on, not base game) | 48 (U) | Not started. The vision lists single-player career with progression, but no milestone or roadmap line plans it | Differentiator | RG-06 |
| Timetables and tours | TT1 (verified), TT2 (verified) | 17, 18, 19 (V) | Done for timetable and tour data (M-0099, M-0112). Day-type and night circuits not started | Parity gap for circuits | GAP-07 |
| Timetable and route editing | TT3 (community) | 21 (U) | Not started. Editor mode covers timetables in its planned phases (M-0051, phase E5) | Parity gap (deferred) | GAP-20 |
| Chronology and temporary changes | TT4, TT5 (verified) | none | Partial. Overlays for the session's map date are applied, and a date change rebuilds the affected tiles (`chrono.rs`). The menu control that picks the date was not checked | Unresolved | none |
| Timetable generator and AI deployment (paid tool) | TT6 (verified, paid add-on) | none | Out of scope. Paid add-on tool | Out of scope | none |
| AI road and rail traffic | TR1, TR2 (verified) | 19 (V), 22 (V, store claim contested), 24 (V, status) | Partial. Road traffic runs (M-0049). AI does not react to the player (GAP-13). Scheduled AI trams and trains done (M-0112). Bus-priority signals not found | Parity gap | GAP-13, GAP-11 |
| Pedestrians | TR3 (community) | none | Partial. Walking done (M-0050). Crossings and yielding not built | Parity gap | GAP-12 |
| Map and terrain editor | M1, M2, M3 (verified), M5 (community, SDK manual unread) | 25, 26 (V, free terrain forming not implemented) | Not started for authoring. Editor mode is specified with phases E1 to E6 open (M-0051). Runtime terrain carving exists | Parity gap (deferred) | GAP-20, M-0051 |
| Map loading and streaming | M4 (verified) | none | Done. Tile streaming (M-0089, M-0114 in progress) | Done | none |
| Content tools, workshop and sharing | MM2 (verified, DLL hook, out of scope), MM3 (community), MM4 (unverified) | 27, 28, 44 (V) | Partial for tooling: script generator and visual script editor specified (M-0055, M-0056). No workshop. Community distribution planned in Phase 6 | Parity gap; sharing is differentiator | GAP-20, RG-10 |
| Weather and seasons | W1 (verified) | 32, 33 (U, contested) | Done with a caveat for OMSI presets (weather doc). Snow and wetness partial (M-0121). Real weather is research only (M-0116), not an OMSI feature | Parity gap (partial) | GAP-15, ISS-016 |
| Day, night and lighting | W2, W3 (verified) | 31, 34, 35 (V) | Done with a caveat. Sun and moon, calendar, night maps (M-0045). Lighting doc contradicts code | Done | ISS-017, ISS-018 |
| IBIS and cab displays | C1 (verified), C2 (community) | none | Done (M-0099, M-0093). Destination display partial (M-0100) | Done | none |
| Failures, malfunctions and repair | F1 (verified), F2 (community, can be turned off) | 40 (U, damage and maintenance) | Not started. No failure injection, repair or maintenance option | Parity gap | GAP-03 |
| Wash and tank at a depot | none in inventory (matrix H) | 40 (U) | Not started for wash. Tanking partial (F12 only) | Parity gap | GAP-19 |
| Accidents and collision response | F3 (community) | 2 (V, collision physics not implemented) | Partial. First version covers static objects, AI vehicles and ground steps. Trailers pushed by the head only | Parity gap | RG-03 (see ISS-011) |
| Electric and fuel | none in inventory | none | Partial. Fuel model in progress (M-0123). Electric state of charge not found | Parity gap | RG-01 |
| Driver aids (ABS, ASR) | none in inventory | none | Not modelled. `brakes.rs:8` states there is no ABS. The scripts were not checked | Parity gap (provisional) | RG-02 |
| Keyboard reference and key list | K1 (community) | 39 (V, keyboard and controller settings) | Partial. Help table omits live bindings; rebinding first slice done (M-0142) | Parity gap | ISS-015, GAP-18 |
| Gamepad, wheel and axis settings | K3 (community) | 39 (V) | Partial. Keyboard rebinding done. Gamepad and wheel open (M-0142, phase 7) | Parity gap | RG-08 |
| Hardware desk interface | none | 37 (V, plugin interface for hardware) | Not started. The control protocol (ADR-0006) is a loopback JSON-RPC interface for tooling, not a device plugin API | Differentiator | RG-08 |
| TrackIR head tracking | K2 (verified) | none | Not started. No code or docs hits | Parity gap (owner decision) | RG-04 |
| Virtual reality | none | 38 (V) | Not started. No code or milestone | Differentiator | RG-09 |
| Multiplayer and co-op | MM1 (unverified; absent in base game per research) | 20, 41, 42, 43, 46 (V, C) | Not started. Long-term item in the vision, no milestone | Differentiator | RG-05 |
| Scenarios and challenges | none in inventory | 12, 18, 23, 47 (V) | Not started beyond free drive and sandbox. Scenario editor is a vision item only | Differentiator | RG-07 |
| Real-time handling of wrong operations | none | 49 (V, planned) | Not assessed. The inventory does not say what it means | Unresolved | none |
| Out-of-scope content and criticism | OMSI paid tools, DLL hook (see the rows above) | 4, 5, 7, 8, 29, 30 (content); 36, 45 (reviewer criticism, not features) | Out of scope under ADR-0002 and the paid-content rule | Out of scope | none |
| Interior climate | none in inventory | none | Not started. Heater and HVAC not confirmed on screen (matrix C) | Parity gap | GAP-17 |

Counts per outcome, across the 97 inventory entries: 27 done, 31 open with an existing GAP or ISS id, 23 open under an RG item, 11 out of scope, and 5 unresolved. Items RG-01, RG-02 and RG-12 come from the matrix rather than the inventories, so they are not in these counts.

## 4. Gap register

The register lists each gap that neither a GAP nor an ISS entry covers. IDs are provisional. Severity uses the scale of the issue register: blocker stops a correct release or a claim the project relies on, major is a defect or gap a player, tester or maintainer would hit, and minor is a limited gap or hygiene. No item in this register is a blocker. Only RG-06 is rated major, and that is because the vision commits to it with no milestone.

**RG-01. Electric bus state of charge.** Electric buses are in the installed OMSI content. The fuel and battery notes record that the variable carrying an electric bus's state of charge is not in the predefined list, and the open item is still listed. OPTS therefore cannot yet show or act on a charge level for electric buses. Evidence: `docs/src/simulation/fuel-battery.md` (open item), the matrix's section I, and the OMSI corpus electric-bus content as cited in the 2026-11-18 matrix (not re-checked in this pass). Severity: minor. Suggested milestone: M-0123 (fuel and battery, in progress).

**RG-02. ABS and ASR driver aids.** The brake model states outright that it has no ABS (`crates/opts-sim/src/vehicle/brakes.rs:8`). The wheel-slip code exists, but the grep for ABS or ASR logic found nothing. The vehicle-functions catalogue (`docs/src/simulation/vehicle-functions.md`) lists ABS and ASR as functions of the vehicle. Whether the OMSI vehicle scripts supply these aids is the open question. If they do, OPTS may already run them and the gap is documentation only. If they do not, the gap is a player-visible handling difference on slippery roads, and the severity rises to major. The scripts were not read in this pass. Severity: minor (provisional). Suggested milestone: new, proposed; or M-0097 (physics contract) if the contract takes it.

**RG-03. Collision response beyond the first version.** OMSI 2 lists accident handling as community guidance (F3), and LOTUS states that its collision physics are not yet implemented. OPTS has a first version (`docs/src/simulation/collisions.md`) that covers static objects, AI vehicles and ground steps, and it pushes trailers from the head only. The trailer seam test (ISS-011) is a separate question about the joint, not about collision response. What remains open is the response to people, to the player's trailers, and to vehicles beyond the first version. Severity: minor. Suggested milestone: new, proposed.

**RG-04. TrackIR head tracking.** OMSI 2 lists TrackIR 5 support as verified. OPTS has no code or docs for head tracking, and the 2026-11-18 matrix placed it under "out of scope?" without an owner decision. The gap is only a player gap for owners of the hardware. Severity: minor. Suggested milestone: none. An owner decision is needed first.

**RG-05. Multiplayer and co-op dispatch.** LOTUS offers multiplayer tours, a control centre, a signal box, radio communication and online co-op, and its reviewers criticise the multiplayer performance. OMSI 2's base game has no multiplayer according to the research. OPTS lists multiplayer with dispatcher and driver roles in its vision as a long-term item, with no milestone. Severity: minor. Suggested milestone: none (vision only).

**RG-06. Career and progression.** The vision commits to a single-player career with progression. OPTS has no milestone, roadmap line or design for it. OMSI 2's career (earning money, buying depots and buses, maintaining vehicles, shift schedules, rankings) is in the paid add-on Bus Company Simulator, not the base game, so this is not an OMSI parity gap. LOTUS has challenges that grow over time and an unverified career. The gap is a planning gap: the vision promises a feature that nothing plans, and a player would expect it. Severity: major. Suggested milestone: new, proposed, with an owner decision on whether career is in scope at all.

**RG-07. Scenarios, challenges and scoring.** LOTUS offers a tight-schedule challenge, a difficult-traffic challenge, special passengers and challenges that grow over time. OPTS has free drive and a sandbox, and the vision adds a scenario editor for community challenges, but there is no challenge or scoring mode. The OMSI base game does not show this as a feature in the inventory. Severity: minor. Suggested milestone: new, proposed, tied to the vision's scenario editor.

**RG-08. Input devices beyond the keyboard, and a hardware desk interface.** OMSI 2 lists steering-wheel and joystick axis settings, including a wheel setup guide (K3). LOTUS lists keyboard and controller settings (39) and a plugin interface for a driver's desk (37). OPTS has a keyboard rebinding first slice (M-0142). Gamepad and wheel rebinding are open in that milestone, and a device plugin interface does not exist. The control protocol (ADR-0006) is for tooling and is not a device interface. Severity: minor. Suggested milestone: M-0142 for gamepad and wheel; a new proposed item for the desk interface.

**RG-09. Virtual reality.** LOTUS lists VR support. OPTS has no VR code, no milestone and no decision. Severity: minor. Suggested milestone: none. An owner decision is needed, and the likely answer is out of scope for now.

**RG-10. Community content distribution and sharing.** OMSI 2 has an add-on and mod ecosystem (MM3), and its Steam Workshop support is unverified (MM4). LOTUS lists a Steam Workshop for extra content (44) and a level editor for sharing (28). OPTS plans a modding ecosystem in Phase 6, which is planned but not started. The gap is the sharing and distribution layer, not the content formats. Severity: minor. Suggested milestone: Phase 6 (existing, status planned).

**RG-11. Vehicle component swaps.** LOTUS lists swappable vehicle modules for displays, on-board computers and ticket printers (11). OPTS has a vehicle profile override in Phase 14 (M-0135, proposed), which covers sound, script, constant and mesh swaps, but not component swaps for displays or ticket hardware. Severity: minor. Suggested milestone: M-0135, if its scope is extended; otherwise new, proposed.

**RG-12. Stop-dependent fares and fare computation.** The 2026-11-18 matrix records fare computation with stop-dependent prices as missing, and marks it as rare and deferred in the vehicle-functions catalogue. The matrix also records that OMSI prices rise across 1986 to 1994, as the manual is cited there (section 4.2.3, not re-read in this pass). GAP-06 covers the cash desk and change, but not how a fare is computed. Severity: minor. Suggested milestone: M-0156 (cash desk and change, proposed), with fare computation added to its scope.

## 5. Overlaps and reconciliation

### 5.1 Findings already covered by GAP or ISS entries

The following table shows which register entries the research confirms, and which inventory features each one covers. An entry with no feature here is one that neither inventory touches.

| Register entry | Research features covered | Note |
|---|---|---|
| GAP-01 (free-drive boarding) | OMSI P2; LOTUS 13 | Partly closed on master; in-game check open (M-0152) |
| GAP-02 (articulation) | OMSI D2; LOTUS 4 (partly) | Trailer seam and pitch or roll still open |
| GAP-03 (failures) | OMSI F1, F2; LOTUS 40 | Largest player-visible gap. Owner decision still needed on whether breakdowns are wanted |
| GAP-06 (ticketing) | OMSI T2, T3; LOTUS 15, 16 | Cash desk and change in M-0156. Fare computation is RG-12 |
| GAP-07 (timetable circuits) | OMSI TT1 | Day types and night circuits |
| GAP-08 (stops and doors) | OMSI C3; OMSI P1 (partly) | Stop requests, rear and stop-brake doors |
| GAP-09 (rail controls) | LOTUS 3 | Rail lever and direction |
| GAP-10 (trolleybus, pantograph) | LOTUS 3 | Overhead-line input |
| GAP-12 (pedestrians) | OMSI TR3 | Crossings and yielding |
| GAP-13 (AI reacts to player) | LOTUS 22, 24 | Store claim contested by reviewers |
| GAP-14 (stop sequence) | LOTUS 13 | Passenger simulation |
| GAP-15 (snow and wetness) | OMSI W1; LOTUS 33 | Seasons and snow |
| GAP-16 (voices) | OMSI P4 | Boarding announcement |
| GAP-18 (help keys) | OMSI K1 | Same finding as ISS-015 |
| GAP-19 (wash and tank) | LOTUS 40 (partly) | Wash not started |
| GAP-20 (editors) | OMSI TT3, M1, M2, M3; LOTUS 21, 25, 26, 27, 28 | Covers the editor family; M-0051 holds the phases |
| ISS-011 (trailer seam test) | OMSI D2 | Live test covers one trailer and two steer values |
| ISS-013 (door arming) | OMSI P1; LOTUS 14 | Parse-only door tests |
| ISS-015 (help table) | OMSI K1 | Same as GAP-18 |
| ISS-016 (snow order) | OMSI W1 | Snow flag read only at texture load |
| ISS-036 (free drive) | OMSI P2 | Decided and on master; in-game check open |
| ISS-037 (trailer seam) | OMSI D2 | Yaw fixed; pitch and roll open |
| ISS-040, ISS-041 (driving feel) | OMSI D1 (praised physics) | The praise and the open feel issues describe the same model |

GAP-04 (tunnels), GAP-05 (battery), GAP-11 (signal priority) and GAP-17 (cabin climate) have no feature in either inventory. ISS-012, ISS-014, ISS-018 and ISS-020 are also outside the inventories, and so are the safety, process and architecture items.

### 5.2 Stale or conflicting entries

- **GAP-01 is stale.** The 2026-11-18 matrix says free drive boards nobody. The admission rule now in `destination.rs` lets a free-drive bus take anyone whose destination the stop serves. The rule was decided on 2026-11-22 (commit 7e465ce7, an ancestor of master) and its boarding-path tests landed in d834b520. M-0152 stays in progress until the in-game check passes. The register and the matrix should show GAP-01 as partly closed.
- **GAP-06 milestone.** The matrix lists the cash desk under M-0101. The phase 16 pages propose M-0156 for it, and that is the milestone to use.
- **Chronology runtime is missing from the matrix.** The matrix records the map and timetable data as done, but does not mention the runtime overlay switch (`chrono.rs`). The menu control is unchecked, so the item is unresolved rather than done.
- **ABS wording.** The matrix records ABS as not established (N/E). The brake model states explicitly that it has no ABS. The register should say that the omission is documented in code, and that the script side is still unchecked.
- **Manual reference.** The matrix's manual-based statements cannot be checked in this worktree, because the cited PDF is not in it.
- **Fare computation.** The matrix's section E marks fare computation as missing, but no GAP entry carries it. RG-12 fills that gap.

### 5.3 Items found outside the two inventories

The matrix covers several areas that neither inventory lists: electric state of charge (RG-01), ABS and ASR (RG-02), indicators and hazard lights (ISS-020), the heater (GAP-17), the status display (N/E in the matrix), wash and tank (GAP-19), and the bulb failures that depend on the engine-raised triggers (E7). These are not counted in the feature totals. The status display in particular needs a decision: the matrix found no code for it, and the OMSI manual section that describes it was not checked here.

## 6. Clean-room and legal notes

- Nothing was copied from OMSI 2 or LOTUS Simulator. No binaries, no decompiled code, no installed-content dumps and no add-on PDFs were used for this report. The OMSI corpus facts quoted from the 2026-11-18 matrix are counts and file paths, which that matrix states it took read-only.
- The feature descriptions are paraphrases in the report's own words. Store-page and community text is summarised, not reproduced.
- ADR-0002 (clean-room policy) applies to every item in the register. Any future work on RG-01, RG-02 or RG-03 must be specified from OPTS's own measurements and the public documentation, and must not depend on OMSI or LOTUS internals.
- Paid add-on content (the Bus Company Simulator career and economy, the Power Toolkit timetable generator and ticket editor, and the OMSI DLL hook) is classed as out of scope or as a differentiator item, and it is not used as a source for OPTS behaviour. The store-page descriptions of those add-ons were used only to tell the reader what the add-on is.
- LOTUS vehicle packs and maps are not used, consistent with the same policy.

## 7. Open questions and next research steps

1. **OMSI base manual.** Read the base manual in the game's docs folder, with ADR-0002 in mind, to upgrade the door, ticket and timetable entries that rest on community sources. Confirm which manual copy the 2026-11-18 matrix read, because the cited file is absent from this worktree.
2. **ABS and ASR.** Check, in prose only and without reproducing script text, whether the OMSI vehicle scripts supply these aids. The answer decides whether RG-02 is documentation or a major gap.
3. **Electric state of charge.** Identify the variable the electric buses use, which the fuel notes list as open, and decide whether RG-01 belongs in M-0123.
4. **Chronology control.** Find out whether the menu lets a player choose the map date, and whether the 1986 to 1994 fare changes apply. This decides whether the chronology row is done, partial or missing.
5. **LOTUS contested items.** Weather and seasons, the Berlin map, passenger simulation and the engine-lineage question remain contested. Re-check them only if a primary source appears. The LOTUS Rails module contents and the meaning of "real-time support for incorrect operations" are still unknown.
6. **Owner decisions.** Career (RG-06), VR (RG-09), TrackIR (RG-04) and the status display need a yes or no before they are scheduled.
7. **GAP-01 in-game check.** Close M-0152 acceptance item 3 before the register marks GAP-01 as closed.

## 8. Sources

- OMSI 2 store page, Aerosoft product page, the Power Toolkit and Bus Company Simulator store pages, community discussions, a forum thread, a fan wiki, reviews and a SteamDB record, as listed in the OMSI inventory.
- LOTUS Simulator store pages for the base game and the Streets module, a review aggregator, a score site and a third-party description, as listed in the LOTUS inventory.
- OPTS: [parity matrix](omsi-parity-matrix-2026-11-18.md), [feature audit](feature-audit-2026-11-18.md), [issue register](issues-2026-11-18.md), [ROADMAP](../../ROADMAP.md), [vision](../src/vision.md), [clean-room policy](../src/adr/0002-clean-room-policy.md), [control protocol](../src/adr/0006-control-protocol.md), [editor mode](../src/adr/0008-editor-mode.md), [M-0152](../src/roadmap/phase-16/M-0152.md), [M-0156](../src/roadmap/phase-16/M-0156.md), [M-0142](../src/roadmap/phase-7/M-0142.md), [M-0123](../src/roadmap/phase-12/M-0123.md), [M-0135](../src/roadmap/phase-14/M-0135.md), [M-0051](../src/roadmap/phase-6/M-0051.md), [phase 16 page](../src/roadmap/phase-16/README.md), [fuel and battery](../src/simulation/fuel-battery.md), [collisions](../src/simulation/collisions.md), [passengers](../src/simulation/passengers.md), [chronology format](../src/formats/map/chrono.md), [weather](../src/architecture/weather.md).
