# OPTS feature audit, 2026-11-18

Master commit: 295df09f. Scope: driving and vehicles, articulation and the trailer-front joint, doors, fuel, lights, world and traffic, passengers and stops, spatial zones and tunnels, battery and electrics, snow and weather, graphics, overlays, HUD and minimap, controls and help, settings registry, plan-a-drive wizard, CLI and control server.

Method: read-only. The audit read the two scratch inputs (feature-world.md and feature-ui.md under the scratch audit folder), the driving and vehicle findings supplied with the brief, and re-checked a subset of the claims with greps and short reads of source files. Items marked VERIFIED in this report were confirmed by reading code in this pass or in the source audits. Items marked SUSPECTED were not confirmed.

Caveat: nothing was executed by this audit. No cargo build, no test run, no game run. "Works and tested" below means a unit or integration test exists in the tree; it does not mean the test was run at this commit. Every on-screen claim is unverified.

## Executive summary

1. The trailer-front seam is described as merged and working on the board, but the seam is unverified at any angle, the only live test covers one trailer at two steer values, and the pitch and roll seam is open.
2. A free drive (the wizard default, "No trip") takes no passenger on at any stop, because boarding needs a bus destination that a free drive never sets.
3. The door pneumatic arming that the brief says is claimed is not in the Rust code; door open state is read from script variables, and the arming logic (if it exists) lives in scripts that were not checked.
4. Tunnel and spatial-zone detection never runs in play: the tile loader passes an empty spline list, so every tile has no zones.
5. The Help controls table omits several live bindings (F3, Shift+F7, Ctrl+F7, Shift+F8, F11, Shift+F11, the arrow aliases), and its test only checks that listed keys exist.

Also serious, though below the top five: the lighting doc contradicts the bloom and mirror ladder in code.

## Verdict table

| Feature area | Status | Evidence |
|---|---|---|
| Driving and vehicles (controls, drive model) | Works but partly untested | Code and unit tests; game not run |
| Articulation and trailer-front joint | Partly done. Seam not closed | Code; one live test (`crates/opts-bevy/tests/trailer_front_live.rs`); bellow tests compare against a rigid helper |
| Doors (open state, boarding, arming) | Claimed but not located in Rust (arming). Readback works in code | Code; two parse-only tests |
| Fuel (depletion, torque cut, refuel) | Works and tested for depletion and refuel. Refuel-when-stopped and script authority open | Unit tests (`opts-sim/src/vehicle/fuel/tests.rs`, `opts-game/src/control_fuel.rs:118`, `opts-bevy/tests/tank_percent_feed.rs`) |
| Lights and indicators | Partly done. No indicator or blinker tests | Code; in-game screenshot check open (M-0045) |
| World and traffic (AI traffic, pedestrians) | Partly done. Not assessed in code, only against board text | Board text only |
| Passengers and stops | Partly done. Works with a chosen trip, not with a free drive | Code and unit tests on the table; no boarding test with no trip |
| Spatial zones and tunnels | Claimed but not implemented in play. Thresholds unvalidated | Code and synthetic tests; detector not wired |
| Battery and electrics (battery switch) | Works in code path, untested. Off-trigger behaviour suspected broken | Code; no test drives the battery-off script path |
| Snow, wetness, weather, puddles | Works but untested on screen. Snow on scenery depends on load order | Code and value tests |
| Graphics and bloom | Works, lightly tested. Bloom Off in every automatic row | Code; one test fn in `bloom.rs` |
| Overlays (Off, Simple, Full) | Works and unit-tested. F10 panel directory has no tests | Code and unit tests; rendering unchecked |
| Debug overlay (M-0049) | Partly done. Reopened 2026-11-15 | Code; 0 tests in `overlay/` |
| HUD and minimap | Works, partly tested. Rotate and speed zoom missing from Help | Code and 7 test fns |
| Controls and help | Broken as documentation. The Help table omits live bindings | Code; help test checks only forward direction |
| Settings registry | Works. Keys have readers (grep-level) | Code; settings screen tests cover storage |
| Plan-a-drive wizard | Works, partly tested. Trip step logic tested; UI untested | Four trip-step tests |
| CLI and control server | Not assessed | Test count only (37 fns) |

## Findings

### FEAT-01 (MAJOR, VERIFIED in part): trailer-front seam recorded as done, not closed

- Claim: the board's trailer-front joint entry (docs/dev/BOARD.md line 555, "stage 3, merged to master as 3420545e") and the supplied brief's quote of a "stages 1-3 on master" line. The quoted wording was not found verbatim in BOARD.md; the nearest entry is line 555.
- Evidence: M-0053 (docs/src/roadmap/phase-4/M-0053.md) keeps "Trailer-end seam verified at all turn angles and speeds, with a before/after comparison" open (line 89), and the pitch and roll seam open (line 90). The correction in that item says the earlier 12-degree frame was taken while the joint was inert. VERIFIED: `trailer_front_transform` reads `sim.consist_alpha_rad(0)` and `sim.joint_angles_deg(0)` (crates/opts-bevy/src/articulation/trailer_joint.rs:42-43), which is the fix in 17473f64.
- Why it matters: the board tells the next agent the joint is done. The evidence is one frame at about 25 degrees, no before/after comparison, and placeholder steer values. A bus visibly broken at other angles would not be caught.
- Next action: record the seam as open on the board and in the ROADMAP line. Before closing it, capture a before/after series over several turn angles and speeds, and assert the pivot at H. Replace the placeholder steer values with measured ones.

### FEAT-02 (MAJOR, VERIFIED by the driving audit; files confirmed to exist): seam tests check little

- `crates/opts-game/tests/trailer_joint_bellow/` (main.rs, rigid.rs) checks the mesh remap against the rigid helper at 1e-4. It does not drive the live system, so it passes whether or not the joint works.
- The only live test, `crates/opts-bevy/tests/trailer_front_live.rs`, is a real regression test (the driving audit confirmed it failed before 17473f64). Its limits: one trailer, two steer values (+/-0.6), one steady turn, yaw only at 1e-3 rad, no pivot-at-H assertion, no pitch or roll, flat ground injected through `apply_ground`.
- Next action: add a live test for a multi-trailer consist, a range of steer values and speeds, a pivot-at-H assertion, and a non-flat ground case.

### FEAT-03 (MAJOR, VERIFIED that the Rust side has no arming; SUSPECTED that the claim is wrong): door pneumatic arming not located

- The brief claims door pneumatic arming. A grep for pneumatic, arming and armed in opts-game, opts-sim, opts-bevy and the world crate found only two doc comments: `crates/opts-sim/src/vehicle/brakes.rs:2` and `crates/opts-sim/src/vehicle/drivetrain.rs:5`. Neither is about doors.
- `crates/opts-game/src/in_game/omsi_world/vehicle/doors.rs` contains only door-point parsing (`door_points`, `door_points_from_bytes`, `attach_door_points`, `attach_unit_door_points`, at lines 13, 47, 59 and 85).
- `vehicle/doors/tests.rs` has two tests, both on parsing (`the_doors_are_the_entries_then_the_exits_that_have_a_pathpoint` at line 25, `a_bad_file_gives_no_doors` at line 39). Neither tests opening, closing or arming.
- Door open state is read from script variables: `crates/opts-game/src/passengers/system/doors.rs:6-17` reads `pax_{kind}{n}_open` from the vehicle engine. So any arming logic is in the vehicle `.osc` scripts, which this audit did not check.
- Status: the Rust door code is readback only, with no test of it. Whether arming exists is unknown.
- Next action: find the board or roadmap entry that makes the arming claim (not found in this pass). Read the door scripts for the arming logic, which the user has allowed as prose. Add a test that sets `pax_entry{n}_open` and checks the passenger code's response.

### FEAT-04 (MAJOR, VERIFIED): Help controls table omits live bindings

- Missing from Help: F3 (`rail_switch_cab`, read at `crates/opts-bevy/src/cab_switch.rs:51`); Shift+F7 and Ctrl+F7 (`minimap_rotate`, `minimap_speed_zoom`, read at `crates/opts-game/src/hud/minimap/input.rs:16,21`); Shift+F8 (`switch_vehicle_prev`, `crates/opts-game/src/session_vehicles/mod.rs:78`); F11 and Shift+F11 (lanes and agent ids, `crates/opts-game/src/in_game/omsi_world/traffic/lane_overlay.rs:45`); the ↑ and ↓ throttle and brake aliases (`crates/opts-bevy/src/vehicle_sim/input.rs:53,62`).
- VERIFIED in `crates/opts-game/src/help/controls/panels.rs`: the F7 row (line 48) lists only `minimap_toggle`, and the Shift+F12 row (line 121) lists only `KeyCode::F12`, so the chip text is not compared with the chord.
- Why no test catches it: `crates/opts-game/src/help/controls/tests.rs` (line 33) checks that each listed key appears as the substring `KeyCode::X` in its source. It does not check that each handler key is listed, and it does not check the chip text.
- Next action: add the missing rows to `help/controls/driving.rs` and `panels.rs`. Add a reverse test that scans `action_keys::ACTIONS` and the handler files, and fails on any unlisted action.

### FEAT-05 (MAJOR, taken from the world audit, not re-read in this pass): free drive boards nobody

- `admits(dest, bus)` needs both destinations to be `Some` and equal (`crates/opts-sim/src/passengers/destination.rs:88`). A bus with no timetable run has destination `None` (`crates/opts-game/src/passengers/destinations.rs:56-60`), so no passenger boards it.
- The wizard default is "No trip" (`docs/src/roadmap/phase-9/wizard-and-loading.md:96`). So the default drive takes nobody on at any stop.
- The docs state the rule (`docs/src/simulation/passengers.md:16`), but the ROADMAP line for M-0101 reads as done with no caveat about free drives. The experiment protocol lists this case as unknown (`docs/src/architecture/omsi-experiment-protocol.md:47`, R6).
- Next action: decide the free-drive rule (board anyone whose destination the stop serves, or pick a trip for every drive) and record it. Add a boarding test with `bus = None`. State the limit on the M-0101 card.

### FEAT-06 (MAJOR, taken from the world audit, code path partly confirmed): tunnel and zone detection never runs in play

- `sync_tile_zones` calls `detect_tile_zones(tile, &[])` (`crates/opts-game/src/in_game/omsi_world/tile/zones.rs:14-22`) with an empty spline list. The doc comment says every tile has no zones until the feed is wired.
- The covered-section thresholds (3.5 m height, 2.0 m width, 15 m length, `crates/opts-bevy/src/spatial_zones/ceiling.rs:16-20`) are tested only with constructed profiles (`ceiling_tests.rs:17-93`). No real tunnel was measured. The labelling and precision/recall work for M-0115 is open.
- The board marks M-0115 partial, which is honest. The zone overlay and `zones.query` answer empty in play.
- Next action: feed the parsed splines from the tile loader. Label the sections of a real tunnel (the Spandau U-Bahn is named in the board) before trusting any threshold.

### FEAT-07 (MAJOR, VERIFIED): lighting doc contradicts the code

- `docs/src/architecture/lighting-system.md:428`: row 0 says "bloom if on". The ladder sets `bloom: BloomLevel::Off` in every row (`crates/opts-bevy/src/light_quality/ladder.rs:47`), and bloom is reachable only in Custom mode.
- `lighting-system.md:444` says bloom "is not" built. Bloom is built (M-0122, 2026-11-16).
- The mirror column gives 0 mirrors for levels 4 and 5. The code sets `mirror_count: 1` and `mirror_every: 4` for levels above 3 (`ladder.rs:44-45`), and the code's own comment says "one picture every 4th frame".
- Next action: correct lines 428, 444 and the mirror column to match `ladder.rs`.

### FEAT-08 (MAJOR, taken from the world audit; SUSPECTED for the script behaviour): battery-off path is untested and may be broken in four vehicles

- The code produces `cp_batterietrennschalter_toggle_off` for the key and `kw_batterietrennschalter_off` for the hot spot. The name `cp_batterietrennschalter_off` in M-0123 (`docs/src/roadmap/phase-12/M-0123.md:23`) does not match any code name.
- No Rust code handles a battery `_off` trigger. The claim that electrical outputs go to zero with the switch off rests entirely on the scripts. No test checks it.
- `docs/src/quality/content-issue-registry.csv` (lines 921, 938, 950, 966) lists an unbalanced `{if}` in the battery trigger of four HH109 `cockpit.osc` files. SUSPECTED: this may stop the off branch from running. Not checked against the script engine.
- The board's M-0123 entry lists script authority as open but does not mention the battery-off trigger.
- Next action: add a script-rig test that presses the toggle and its release, then asserts `elec_busbar_main` is zero. Check the four `cockpit.osc` files against the parser. Name the battery-off trigger as an open question on the M-0123 card.

### FEAT-09 (MAJOR, taken from the world audit): snow on scenery depends on load order

- The snow flag is set each frame (`crates/opts-game/src/in_game/omsi_world/scenery_objects.rs:146`, `crates/opts-bevy/src/season.rs:18`) but read only when textures load (`season.rs:23-26`). A texture already loaded keeps its old variant, so scenery shows snow only if it loaded after the weather changed.
- The test `snow_variants_come_first_only_under_snow_cover` (`season.rs:68`) checks folder order only.
- SUSPECTED: `graphics.snow_intensity` scales the drawn flakes (`weather/effects.rs:60-70`), and the doc comment at `effects.rs:32` says it also scales the cover. The cover flag does not read it.
- The board's M-0121 entry says "on-screen evidence open" and does not mention the load-order limit.
- Next action: refresh loaded textures when the snow flag changes, or record the limit on the card. Correct the comment at `effects.rs:32` or wire the setting in.

### FEAT-10 (MINOR, VERIFIED): debug overlay partly done and untested

- M-0049 was reopened 2026-11-15; lane drawing and other scope is open. The F10 developer panel is in `crates/opts-game/src/in_game/omsi_world/overlay/` (panel.rs, position.rs, readout.rs, wheel_vars.rs). A grep found zero test functions in that directory.
- Next action: finish the M-0049 scope or re-label the board entry. Add tests for the overlay directory or record the gap.

### FEAT-11 (MINOR, VERIFIED): bloom needs HDR and the settings page does not say so

- Bloom needs HDR (`bloom.rs:11`; HDR defaults on). The Low and High intensities (0.03 and 0.10, `bloom.rs:24-29`) are unmeasured, as `levels.rs` admits. Bloom has one test function.
- Next action: note the HDR dependency on the bloom row in the settings docs and the registry help text.

### FEAT-12 (MINOR, VERIFIED): lights and indicators lack tests

- No indicator or blinker tests were found. M-0045 keeps the in-game screenshot check and the delay open (BOARD lines 180, 207).
- Next action: add a unit test for indicator state and timing, and record the screenshot check as the open item.

### FEAT-13 (MINOR, VERIFIED): board names a branch that does not exist

- BOARD.md line 3 says the articulation sign flip is on `agent/signfix`. That branch does not exist (`git branch -a` finds none). The change is on master as 03a9306e, and 0eb12094 merged the alpha sign test. OMSI agreement on the sign is still open (M-0053 line 178).
- Next action: correct line 3 to cite 03a9306e.

### FEAT-14 (MINOR, SUSPECTED): Back and Escape pause guard is hard to read

- `crates/opts-game/src/pause/input.rs:24-26` opens the pause menu on `UiAction::Back` when Escape was just pressed or Backspace was not. This looks intended (Backspace is the drive's key), but it may open the menu on a Back from another source.
- Next action: add a test for a Backspace-only Back.

### FEAT-15 (MINOR, VERIFIED by the UI audit): minimap rotate and speed zoom absent from Help

- Covered by FEAT-04. Listed here because the minimap itself has 7 test functions and works.

### Not assessed

- CLI and control server: `PROTOCOL_VERSION` is 1.36.0 (`crates/opts-cli/src/protocol.rs:7`). The method list was not checked against the docs.
- AI traffic (M-0049, M-0111), pedestrians (M-0050) and real-weather research (M-0116): checked only against board text.
- The tree-wind race fix (BOARD line 555 area) rests on one clean run. Outside this audit's scope.

## Process lesson: features marked done on evidence that tested nothing

The trailer-front joint is the clearest case. `trailer_front_transform` read a missing unit, so the joint was inert on one-trailer buses until 17473f64. The earlier 12-degree frame was taken with the joint inert, and the board still recorded stage 3 as merged. The mesh-remap test compared the bellow against the rigid helper, so it passed with the joint off. The one test that caught the bug was written late.

Other instances, named plainly:

- M-0101 reads as done for passenger boarding. The free-drive case, which is the default, is not covered by any test (FEAT-05).
- M-0115 tunnel detection thresholds were validated only on constructed profiles, and the detector is not wired (FEAT-06).
- M-0124 is marked complete while its dynamic-mass item is unchecked, and the board records the closure as a limit.
- M-0123 lists script authority as open but does not mention the battery-off trigger (FEAT-08).
- The Help test checks substrings and forward direction, so it certified a table that omits live keys (FEAT-04).
- The lighting doc was never updated when bloom was placed in Custom only (FEAT-07).

The rule that would have caught these: a feature is done only when a test drives the live path and fails when the feature is removed.

## Recommended order of work

1. Correct the board and ROADMAP for the trailer-front seam, M-0101 free drives and M-0123 battery-off, so the status reflects evidence (FEAT-01, FEAT-05, FEAT-08).
2. Decide the free-drive boarding rule and add its test (FEAT-05).
3. Fix the Help table and add the reverse test (FEAT-04).
4. Find the door arming claim; read the door scripts and add a readback test (FEAT-03).
5. Wire the tile spline feed into zone detection, then label one real tunnel (FEAT-06).
6. Add a live multi-trailer seam test and capture the before/after series (FEAT-02, FEAT-01).
7. Correct the lighting doc and the stale board reference (FEAT-07, FEAT-13).
8. Fix snow load order or record the limit (FEAT-09).
9. Finish or re-label M-0049 and add overlay tests (FEAT-10).

## Appendix: sources used

- Scratch inputs: `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/audit/feature-world.md`, `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/audit/feature-ui.md`.
- Driving and vehicle findings supplied with the brief.
- Re-checked in this pass: `crates/opts-game/src/in_game/omsi_world/vehicle/doors.rs` and `doors/tests.rs`; `crates/opts-game/src/passengers/system/doors.rs`; grep for pneumatic and arming across the game, sim, bevy and world crates; `crates/opts-bevy/src/articulation/trailer_joint.rs`; `crates/opts-game/src/help/controls/panels.rs` and `tests.rs`; `crates/opts-bevy/src/light_quality/ladder.rs`; `docs/src/architecture/lighting-system.md`; `docs/dev/BOARD.md` (lines 3, 555); `docs/src/roadmap/phase-4/M-0053.md`; git branch list; commit 03a9306e.
- Not re-read in this pass (taken from the scratch audits): passenger, destination and zone code; battery-off script behaviour; snow code; the tests under `crates/opts-game/tests/trailer_joint_bellow/`.
