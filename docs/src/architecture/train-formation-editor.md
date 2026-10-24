# Train Formation Editor (design)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft - phase B, awaiting approval |
| **Milestone** | [M-0106](../roadmap/phase-10/M-0106.md) (first deliverable); follow-ups in section 11 |
| **ADR** | [ADR-0013](../adr/0013-train-formation-model.md) |
| **Research** | [train-formation-research-2026-10-15.md](../meta/train-formation-research-2026-10-15.md) (evidence tags as there) |
| **Builds on** | [rail vehicles](../simulation/rail-vehicles.md), [chain assembly](chain-assembly.md), [articulation physics](articulation-physics.md), [HUD registry](hud-registry.md), [session save](session-save.md) |

## 1. Goal and non-goals

A player composes a train from rail vehicles in the spawn menu (add, remove, reorder, turn around), spawns it as a coupled consist on a track and drives it from the first car. Presets come from the installation's `.zug` files; custom formations are saved and reloaded.

Non-goals of this design: AI trains from formations, cab switching, a 3D preview, rigid-body couplers, writing `.zug`, a track picker, signals and timetables.

## 2. Findings that shape the design

1. A `.zug` is a list of `(vehicle file, 0|1)` pairs; the vehicles are `.ovh` (100 %), almost all self-contained cars with two coupling points and no `[couple_back]` (research 1.1-1.4). That is already a formation model; nothing needs to be invented for the data.
2. Part-chain vehicles (the NF6D: 10-40 files per vehicle) cannot be used as "one car per `.bus`"; they are *units* with a fixed inner chain (research A5).
3. The rail model places cars by arc length on the track and has one speed; mass is the head's only (research A4, A7). The bus articulation joint is not involved.
4. Spawn: one `.bus` head near a track works; everything else does not exist (research A3).

## 3. Data model

In `opts-core` (`formats::trains`, no Bevy, no I/O except the parser's `&[u8]` input):

```rust
/// A train: cars front to back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainFormation {
    pub name: String,
    pub segments: Vec<FormationSegment>,
    pub source: FormationSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormationSegment {
    /// A vehicle file relative to the OMSI root (`Vehicles\Folder\Car.ovh`, backslashes kept as written)
    /// or a vehicle group name that the map resolves (`.zug` allows both).
    pub vehicle: VehicleRef,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VehicleRef { File(String), Group(String) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Orientation { #[default] Forward, Backward }   // .zug: 0 / 1

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormationSource {
    Custom,
    /// Loaded from this `.zug` (relative path); edits keep the link as `template`.
    ZugFile(String),
}
```

Differences from the brief's sketch, with reasons:

- `is_player_controlled` is **not a field per segment**: it would allow zero or several controlled cars. The controlled car is derived: the **first segment** in v1 (B7). When cab switching exists the formation gets one `controlled: usize`.
- `vehicle_path: PathBuf` became `VehicleRef`: a `.zug` entry can be a group name (18 files), and OMSI paths are case-insensitive backslash strings, not host paths. Resolution to a file happens at validation time with the installation index (`find_file`), never in the model.
- The model is pure data; **validation** needs the installation and is a separate function (section 6).

Parser: `TrainFormation::from_zug(bytes: &[u8], name, path) -> Result<(TrainFormation, Vec<ZugNote>)>` with: CP1252/UTF-8 fallback, CRLF, missing final newline, trailing blank lines trimmed, an odd last line reported as a note and dropped, orientation values other than `0`/`1` an error, a file that looks like a script (`{init}`, `'` comment lines) rejected by structure (`RUHR.zug`), empty file an error. Not quoted from OMSI: tests use self-authored fixtures.

## 4. `.zug` integration (B2)

| Question | Decision |
| :--- | :--- |
| Parse `.zug` to load presets? | **Yes.** It is the only data source for real consists (222 files) and the format is trivial and fully understood. Spec page `docs/src/formats/text/zug.md` (the existing `misc-types.md#zug` section is replaced by a link to it). |
| Write `.zug` for custom formations? | **No, deferred.** The format has no name, template, version or group semantics beyond what OMSI reads, and writing into `Trains/` would touch the OMSI installation (forbidden). Export to a user-chosen folder can follow later (two lines per car). |
| What does OPTS use for custom formations? | TOML in the user data directory (section 8). |
| Group-name entries | Parsed and kept as `VehicleRef::Group`; in v1 the editor shows them but **validation rejects spawning** such a formation (no map groups outside a session); later: resolve against the map's `ailists.cfg`. |

## 5. Physics model (B3)

**Decision: Option C, in its smallest form. Placement stays kinematic (the existing rail model); mass, brake and traction are summed per car; coupler dynamics are not modelled and have a defined hook.** The reasoning and the alternatives are in ADR-0013; the essentials:

| Aspect | Model |
| :--- | :--- |
| Position of every car | **Existing rail model**: the head's front bogie is the train point on the track (`Train`), each car sits at `behind` metres along the same track and finds its pose from its two bogies (`unit_frame`). Exact in curves and points, no yaw state. **The bus articulation joint (`TrailerKinematics`) is not reused**; the task's question "kinematic joint or rigid-body coupling" is answered *for rail*: neither; arc-length following, which is kinematic and already implemented. `articulation.rs` is not modified. |
| Longitudinal dynamics | `RailDynamics` stays one body, but built from a `ConsistLoad { total_mass_kg, powered_mass_kg, braked_mass_kg }` instead of the head's mass: running resistance and gradient act on the total mass, brakes on the braked mass, traction and power on the powered mass. |
| Powered mass | the **first car and its own chain**, every **multi-file car** (a multiple unit with its own parts) and every car the user marks `powered`; single-file cars behind the head are hauled. **Measured (2026-10-15): the vehicle files cannot tell a locomotive from a wagon** — every rail `.ovh` checked (loco, coaches, freight wagon) has a driver camera, `[sound]` and `[script]`, and wagons list driven axles — so the first design rule "has a driver workstation" was replaced. Q-TF-01 stays open for the traction constants in the scripts. |
| Braked mass | all cars. |
| Per-car state | `Vec<UnitLoad>` per car (the type from M-0053 D2 is reused for mass and axle positions); no per-car speed or force. This is the hook for coupler forces: a later model can integrate per-car speeds without a data change. |
| Push, slack, buff, derailment | not modelled. |

The failing case that justifies the change from "head mass only" (without needing rigid bodies): a 20-wagon freight train from `Trains/` (one 84 t loco, 20 wagons of 40 t: 884 t) is simulated today with the head's mass (traction `1.1 m` and brakes `1.3 m` both scale with the mass the model is given): it accelerates at the tram's 1.1 m/s² and brakes at 1.3 m/s², about ten times the physical acceleration for 884 t on 92 kN of traction. The test is part of the milestone (section 9, C2): at equal tractive force the acceleration must scale with `1/total mass` within 2 %, as it does for road consists since D2.

Rigid-body coupling is not chosen because no specific case in the research needs it (push-pull, slack, and collision are *absent* requirements for the first deliverable). The ADR lists the cases that reopen the decision.

## 6. Validation (what the editor and `formation.validate` check)

Pure checks first (no installation), then checks against the installation (vehicle index, manifests):

| # | Rule | Message key |
| :--- | :--- | :--- |
| V1 | at least 2 segments (a single car is a vehicle; "use the Vehicle tab") | `formation-err-too-short` |
| V2 | at most 40 *parts* in total, counting the parts of chain units (the walker's limit); the editor shows the running count | `formation-err-too-many-parts` |
| V3 | every `File` resolves to an existing file (case-insensitive) | `formation-err-missing` |
| V4 | every file is a rail vehicle: `[type]` 2 and `[boogies]` (M-0095 rule) | `formation-err-not-rail` |
| V5 | the first car has a driver workstation | `formation-err-no-cab` |
| V6 | coupling: every car has the coupling point on each side that has a neighbour (`[coupling_front]` / `[coupling_back]`, swapped for `Backward`). A multi-file car (a multiple unit such as the NF6D) **may** be turned: the unit is mirrored as a whole (`consist_plan.rs`) — this was a v1 exclusion in the draft and became possible once the placement was moved into a pure planner | `formation-err-coupling` |
| V7 | no `Group` entries (v1) | `formation-err-group` |
| V9 | the first car runs forward (the player drives it); *Reverse train* turns the whole formation | `formation-err-head-turned` |
| V8 | length: the sum of the cars' lengths is at most the free track behind the spawn point **plus** a forward slide of 200 m; checked at spawn, not in the editor | `formation-err-track` |

`[coupling_front_character]` is ignored in v1 (0 users among the vehicles referenced by `.zug`, research 1.4); a mismatch is never an error. The editor shows all errors at once, never only the first.

## 7. UI (B4)

**Where.** The *Select Vehicle* modal of the Left-Alt menu (`in_game/vehicle_menu.rs`) gets a notebook with two tabs: **Vehicle** (today's page, unchanged) and **Formation**. The new-game wizard (2026-10-24) has a composer of its own: for a rail vehicle the What step offers *Compose a train…*, a dialog (`new_game_screen/formation.rs`) over the step in which cars of the installation are added behind the chosen vehicle, turned, moved and removed (the choice is `NewGameSetup::formation`, cleared when another vehicle is chosen). The drive starts with the whole train: the loading pipeline resolves it like `formation.spawn` does and sets the cars behind the head (`apply_to_start`). The Formation page of the in-game menu stays for changing a train during the session; presets, saved formations and the entry point are only there.

**Widgets (all existing):** `NotebookBuilder` (tabs), `ModalBuilder` (the modal), `RowBuilder`/`StackBuilder`/`CardBuilder` (layout, one card per segment), `ButtonBuilder` (icon buttons: up, down, turn, remove; text buttons: Add, Save, Load, Spawn), `ComboboxBuilder` (Load: saved formations and `.zug` presets, grouped), `TextFieldBuilder` (name), `LabelBuilder`. **No new widget is needed**: the scrollable lists of the modal are built the way the vehicle list is. A reorderable-list widget would be a nicety for later.

**Wireframe (formation tab):**

```text
┌ Select vehicle ───────────────────────────────────────────────────────────────┐
│ [ Vehicle ] [ Formation ]                                                     │
├───────────────────────┬────────────────────────────────┬──────────────────────┤
│ Rail vehicles         │ Formation  [ Name: Three NF6D ]│ Summary              │
│ search [__________]   │ ┌────────────────────────────┐ │ Cars         3       │
│ NF6D_A         12 t   │ │ 1 NF6D_A         → ▲ ▼ ⟳ ✕ │ │ Length       81 m    │
│ NF6D_B_User     6 t   │ │ 2 NF6D_B         ← ▲ ▼ ⟳ ✕ │ │ Mass         ...  t  │
│ BR112          84 t   │ │ 3 NF6D_A         → ▲ ▼ ⟳ ✕ │ │ Parts        27 / 40 │
│ Bpmz291_or     40 t   │ └────────────────────────────┘ │                      │
│ ...                   │ [Add ▶]  Load [ combobox ▼ ]   │ ⚠ messages (all)    │
│                       │ [Save]   Source: custom        │                      │
├───────────────────────┴────────────────────────────────┴──────────────────────┤
│ Spawn at: entry point [ combobox ]   Repaint: (head)        [ Cancel ] [Spawn]│
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Compose:** *Add* appends the selected catalogue vehicle; row buttons move up/down, turn (`→`/`←` shows the orientation), remove. Keyboard: the buttons are focusable like every button; Delete removes the focused row.
- **Rail list:** catalogue items with `class == rail`, **including `.ovh`** (the catalogue marks them not playable, which only hides them from the Vehicle tab; the formation tab uses a separate filter "rail, usable as a car": has `[boogies]` and coupling points or a chain).
- **Preview:** none in v1 (summary only): cars, length, mass, parts, and which car carries the cab.
- **Validation display:** the *Spawn* button is disabled while any error is open; the message list names every error with the row number.
- **Implemented (C3):** the tab sits in the same modal as the vehicle list (the *Vehicle* / *Formation* buttons next to the title). The left column switches between **Rail cars** (all 877 rail vehicle files of the catalogue, including `.ovh`, with search) and **Trains** (the 223 `.zug` consists); the middle column is the train, one row per car with *Up*, *Down*, *Turn*, *Pulls* (cycles automatic / pulls / hauled) and *✕*; the right column is the summary, every problem, and the free entry points. Footer: *Back*, *Reverse train*, *Clear*, *Spawn* (disabled, with the reason shown, until the train is valid and an entry point is chosen). A *Load* combobox and *Save* arrive with C4; the presets are a list rather than a combobox because 223 entries need a search field. The page strings are English like the rest of the vehicle menu, which is not localised yet.
- **Presets:** *Load* lists `.zug` files of `Trains/` (222) and the user's saved formations; loading a `.zug` sets `source = ZugFile` and shows "from Trains/..." (never edited in place).
- l10n keys in a new `formation-en.ftl` / `formation-de.ftl` (the localisation test requires both languages and the core list `opts-core/src/l10n/builtin.rs`).

## 8. Persistence (B6)

**Implemented (C4):** `opts-game/src/formation/store.rs` (`FormationStore`, TOML schema v1 as below, atomic write, unique ids, damaged files listed and kept), protocol `formation.save/list/delete` and a `saved` parameter for `formation.validate/spawn`, the editor's *Saved* list, *Save* (over the file the train came from, else a new file made from its name), *Rename* and per-row delete, and the optional `formation` field of the session vehicle record. The list is newest first (the "recent formations" of the milestone).

Directory: `<user data dir>/formations/<slug>.toml` where the user data dir is the one `save/store.rs` uses (`directories::ProjectDirs::from("org","opts","opts")`, on Linux `~/.local/share/opts`). Atomic write (temp file + rename), like the saves; a file that fails to parse is listed greyed with its error, never deleted.

```toml
format = 1                       # schema version; unknown newer versions are refused with a message
name = "Three NF6D"
template = "Trains\\X10_112_IC-8.zug"   # optional: the preset it started from

[[segment]]
vehicle = "Vehicles\\Tram_NF6D\\NF6D_A.bus"   # OMSI-root-relative, as written in .zug
orientation = "forward"                         # forward | backward

[[segment]]
vehicle = "Vehicles\\Tram_NF6D\\NF6D_B.bus"
orientation = "backward"
```

- A segment can also be `group = "M-Train Cars"` instead of `vehicle` (round-trips `.zug`).
- **Template references:** `.zug` presets are referenced by relative path and **copied** into the TOML on first save (so a later change of the add-on cannot change the saved train).
- **Shared with AI?** No. AI trains come from `.zug` in `ailists.cfg`; sharing user formations with AI is a separate milestone (section 10).
- **Session save:** the spawned formation is stored in the session vehicle's record as `formation: Option<TrainFormation>` with `#[serde(default)]`, so old saves load unchanged and a save made with a formation restores the same train (extends [session-save](session-save.md); M-0069's `SessionVehicleRecord`).
- TOML (de)serialisation lives in `opts-game` (`toml = "0.8"` is already a dependency there); `opts-core` keeps only `serde` derives.

## 9. Spawn flow (B5) and sub-phases

```text
formation (validated) ──▶ resolve each car (index) ──▶ UnitGeometry per car
        │                                              (front/back coupling points, bogie span,
        │                                               mass, driver workstation, chain parts)
        ▼
 place head at the entry point ──▶ rail_player_step snaps head to nearest track (<= 12 m)
        ▼
 behind[i+1] = behind[i] + (f_next.y - b_this.y)       (existing maths; flip: coupling point negated)
        ▼
 train point = head's front bogie; cars at arc length `behind`; ConsistLoad -> RailDynamics
```

- **Head:** the first segment is spawned like today's rail head through `load_player_vehicle` (a `.ovh` head is loaded by the same loader; the playable filter only concerns the *list*). It receives `RailPlayer::new_consist(load, span)` instead of `RailPlayer::new(head_mass, span)`.
- **Following cars:** a new function `spawn_formation_cars(job, commands, ..., root, &[ResolvedCar])` in `omsi_world/rail.rs` loads each car's model like `parse_unit` and reuses `VehicleComposition`/`Part` (`behind`, `half_span`, `reversed`). The coupling maths of `load_spec`/`spawn_trailer_chain` is extracted into one function `behind_of_next(...)` used by all three (the only change to existing code, behaviour-preserving, with a test that the old chains place identically).
- **Initial state:** speed 0, parking brake on, no joint angles (rail has none), the cab of the first car selected, gear neutral as today.
- **Track:** the head is put on the nearest track within **150 m** of the entry point (a single rail vehicle keeps 12 m): entry points are road-oriented and a station track lay 82 m away in the first test (Grundorf, `Nordspitze Bhf.`). Farther than that the head stays a road vehicle and the log names the distance to the nearest track.
- **Direction:** the head faces the entry point's heading; the cars extend *behind* it. If the track behind is shorter than the formation, the head slides forward along the track (at most 200 m, V8) so that all cars fit; otherwise the spawn is refused with the message of V8.
- **No rail on the map:** the formation is not spawned; the editor says so (`formation-err-track`) and keeps the formation. A map without track never offers the Formation tab's Spawn.
- **Orientation of a `Backward` car:** its coupling points are swapped and negated (the maths `load_spec` has for `reversed`); its model is rotated 180° about the vertical axis (`reversed` in `Part` already exists and is used by `rail_player_pose`).
- **Chain units** (NF6D): `consist_plan.rs` places every file of the head's chain and of each car's chain in one pure function (testable without a session). A unit is laid out in its own frame (first file at 0), then put into the train by `t + y` or, turned, `t - y` with every part turned and the order reversed. A chain whose `[couple_back]` link is flagged *reversed* (the NF6D's rear cab seen from behind) ends its unit there and the next car still follows; the unit's rear end is then that turned file's front coupling point.

**Sub-phases (as in the brief):**

| Sub-phase | Content | Test |
| :--- | :--- | :--- |
| C1 | `formats::trains` (model, `.zug` parser, notes), `docs/src/formats/text/zug.md`, corpus test (needs `OMSI_PATH`, skipped otherwise) and fixtures | 204 regular files parse, 18 name files parse as groups, `RUHR.zug` rejected, round trip |
| C2 | resolver and spawner (`ConsistLoad`, `behind_of_next`, `spawn_formation_cars`), protocol `formation.validate`, `formation.spawn` | mass scaling test, placement test, spawn of a 3-car freight formation and an NF6D pair on a map with track |
| C3 | the Formation tab | golden screen for the tab, key and button tests |
| C4 | TOML store, `formation.save/load/list`, session-save field | save/restart/load round trip |
| C5 | verification set from the brief (single NF6D, three-car formation with the middle flipped, save/load, invalid formations, a `.zug` preset) | screenshots (not committed: OMSI textures) |

## 10. Control, AI and limitations

- **Player control (B7):** the first car only; the others are passive (no script host until M-0098). Cab switching is deferred. **v1 limitation, stated:** a formation always drives forward from its first car; to drive the other way the player composes it the other way round.
- **AI (B8):** **separate milestone.** This feature does not unblock AI trains; the bug "`.zug` entries in AI lists are silently ignored" (research A3) is a one-function fix once the parser exists and is proposed as M-0107 so that AI and player share the parser and the resolver.
- **Group entries, `[coupling_front_character]`, `.zug` writing, 3D preview, track picker, cab switching:** deferred.

## 11. Milestones

| ID | Content | Status |
| :--- | :--- | :--- |
| **M-0106** | Train Formation Editor v1 (C1-C5 above) | proposed in this phase |
| M-0107 (proposal) | AI trains from `.zug` (list entries resolve through the parser), shared resolver, group-name resolution against `ailists.cfg` | not written |
| later | cab switching and passive cars' scripts (with M-0098), `.zug` export, coupler dynamics if a reopening case appears (ADR-0013), 3D preview, track picker | not scheduled |

## 12. Risks and open questions

| ID | Question |
| :--- | :--- |
| Q-TF-01 | Which cars are *powered* in OMSI (driver workstation vs a traction constant in the vehicle's `.osc`/`.cfg`)? The cab heuristic is a stand-in; experiment protocol line needed. |
| Q-TF-02 | ✅ Answered 2026-10-15: yes. The rear end of an NF6D unit is the front point of its turned last file; three NF6D units couple and spawn (35 files in all). |
| Q-TF-03 | Orientation flag for chain units: OPTS mirrors the whole unit; OMSI's own meaning is unverified (no `.zug` of the corpus lists a multi-file head). |
| Q-TF-04 | ✅ Max parts: 64 for a formation (40 for a single vehicle). An NF6D head chain has 11-13 files, three units about 35; the first count of 41 per unit was an artefact of following the ring without stopping at the reversed link (fixed in `JobLookup`). |
| Q-TF-06 | ✅ A `.ovh` head loads and drives through `load_player_vehicle` (BR218 on Grundorf). |
| Q-TF-05 | A formation as a *vehicle* in `ailists.cfg` is `Trains\x.zug`; OMSI's AI treats a `.zug` as one vehicle with one script set. The player formation does not have that script structure (M-0098). |
