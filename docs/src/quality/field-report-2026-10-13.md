# Field Report 2026-10-13 (Project Lead)

| Metadata | Value |
| :--- | :--- |
| **Source** | project lead, from hands-on testing of the merged build (`master` at `99a25d4`) |
| **Date** | 2026-10-13 |
| **Status** | 📝 Intake complete; every item has a milestone. Reproduction and root-cause notes are added by the milestone's owner. |

Each reported item gets an ID (R-nn), is traced to a milestone, and is closed only when the milestone's acceptance criteria are met. Wording is the project lead's, shortened.

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-01 | Some **parked car** meshes are not loaded correctly: scrambled, missing or transparent meshes | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) (discovery: P-1014 mesh audit) | 🟢 fixed 2026-10-18 on `agent/mesh-fidelity` (MI-0056–0058) |
| R-02 | OPTS **cannot load water**, although `.water` files lie next to the tiles; they need to be handled | world | [M-0067](../roadmap/phase-10/M-0067.md) | 🟡 water is drawn since 2026-10-16, with a depth tint since 2026-10-18; the map's own water textures are not used (Q-M08) |
| R-03 | The simulation must be **startable without a bus**: the vehicle selection needs a way to load only the map | session / UI | [M-0068](../roadmap/phase-10/M-0068.md) | ✅ implemented 2026-10-13 (`agent/session-vehicles`, verified live) |
| R-04 | OPTS must **track the user vehicles spawned in a session**, let the user **change the active bus** and **despawn any** previously spawned, still existing bus (follow-up 2026-10-13: *despawn*, not *respawn*, was meant; the active bus keeps its respawn) | session | [M-0068](../roadmap/phase-10/M-0068.md) | ✅ implemented 2026-10-13 (`agent/session-vehicles`, verified live) |
| R-05 | **Session persistence**: save the current session and restore it; expandable for weather, timetables and more | session | [M-0069](../roadmap/phase-10/M-0069.md) | ✅ done 2026-10-13 ([Session save](../architecture/session-save.md)) |
| R-06 | **Adhere to the roadmap** again and **expand it** | process | [M-0074](../roadmap/phase-5/M-0074.md) | 🟡 in progress: audit and checks done 2026-10-18 |
| R-07 | The date/time step of a new game must **default to the map's defined time**, otherwise the **current date and time** | UI / wizard | [M-0070](../roadmap/phase-9/M-0070.md) | 🟢 implemented 2026-10-18 (time of day from `laststn.osn`, else the clock; date keeps the map default) |
| R-08 | **HUD rework**: control information (hints, speed, gear, parking brake) in the **bottom-left**, a **mini map bottom-right**, in line with the UI standards | UI / HUD | [M-0071](../roadmap/phase-9/M-0071.md) | ✅ done 2026-10-13 |
| R-09 | The **BRT Citaro** has UV/mesh issues | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-10 | The **control protocol** must allow AI agents to **set settings via the CLI** | tooling | [M-0072](../roadmap/phase-5/M-0072.md) (protocol 1.1.0 has `settings.*`; the exact gap is to be reproduced) | 🟢 implemented 2026-10-18 (protocol 1.7.0: confirm/revert, batch and profile methods, coverage test) |
| R-11 | **Suspension, brakes and more**: every vehicle defines its own mass, axles etc., which must be respected; OPTS provides a world physics system, the vehicles their individual behaviour | simulation | [M-0073](../roadmap/phase-10/M-0073.md) | 🟢 done 2026-10-18 (limits on the spec page) |

## Follow-up findings (2026-10-13, second message)

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-12 | The HUD always shows the fixed string 'Spandau Westerwaldstraße/Siegener Str.' (hard-coded). It must show the **entry point name where the active vehicle spawned**, disappear when the bus has moved away or is despawned, and return with a new spawn | HUD | [M-0075](../roadmap/phase-9/M-0075.md) (data from [M-0068](../roadmap/phase-10/M-0068.md)) | ✅ implemented 2026-10-13 (`agent/session-vehicles`, verified live) |
| R-13 | The loading screen first shows the **MAN SD77** (hard-coded) and switches to the selected vehicle at about 70 %; the selection (or none) should be known from the start | loading | [M-0076](../roadmap/phase-9/M-0076.md) | ✅ implemented 2026-10-13 (`agent/session-vehicles`, verified live) |
| R-14 | Correction of R-04: the wish was to **despawn** any session vehicle, not respawn | session | [M-0068](../roadmap/phase-10/M-0068.md) | ✅ implemented 2026-10-13 (`agent/session-vehicles`, verified live) |
| R-15 | **Bus configurations** (bus, repaint, setvars) that can be saved and quick-loaded | session / UI | [M-0077](../roadmap/phase-10/M-0077.md) | 🟡 in progress 2026-10-18 |
| R-16 | **Setvar implementation**: derive each bus's setvar list from its `.cti` files, the Add-on **PDFs** and the **varnamelists** (lists named *visible* can hold setvars) | vehicles | [M-0078](../roadmap/phase-10/M-0078.md) | 🟢 implemented 2026-10-18 (catalogue, editors and `vehicle.setvars`; configurations in M-0077) |
| R-17 | Several **building meshes are flipped** along an axis and/or have **misaligned textures** | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-18 | Several **lantern posts have scrambled meshes** in the simulation | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-19 | The files of `master` must be present in the project folder on the project lead's machine | process | — (done 2026-10-13: `master` checked out in `/home/louisgoodnews/Dokumente/opts`) | ✅ done |

## Follow-up findings (2026-10-13, third message)

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-20 | A setting **Load all tiles on start**: all map tiles are force-loaded at start and stay loaded; it must also be exposed in the *summary screen* of the new-game menu where the user confirms the start | world / settings / wizard | [M-0080](../roadmap/phase-4/M-0080.md) | 🟡 core implemented 2026-10-18 (wizard switch and memory guard open) |
| R-21 | A setting for the **number of tiles around the centre tile** (the tile the player camera stands on) to load; the default is derived dynamically from the hardware capabilities | world / settings | [M-0080](../roadmap/phase-4/M-0080.md) | 🟡 core implemented 2026-10-18 (wizard switch and memory guard open) |
| R-22 | A new crate **`opts-hardware`** for deep research of the hardware **without spying on the system** | tooling | [M-0081](../roadmap/phase-5/M-0081.md) | 🟡 implemented on `feat/opts-hardware` (Linux verified; macOS/Windows untested; report-dialog preview and GPU benchmark open) |
| R-23 | **Research branch** for leveraging Bevy's **instancing** for repeatedly used meshes | performance | [M-0082](../roadmap/phase-5/M-0082.md) (branch `research/mesh-instancing`) | 🔬 started |

## Follow-up findings (2026-10-13, fourth message)

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-24 | *Settings → Content* shows '679543 Dateien, 75313 Karten, Fahrzeuge und weitere Inhalte'; '75K maps does not add up' (are those tiles?) | settings | [M-0086](../roadmap/phase-9/M-0086.md) | ✅ fixed on `feat/combobox-search`: the number was the total of catalogue entities of all kinds (not maps, not tiles); Settings → Content now shows maps, vehicles, scenery objects, splines and other entries (maps and vehicles equal Home) and explains files versus entries |
| R-25 | A system where **localisation can be added by other human or AI contributors** conveniently, with JSON or other formatted files | UI / i18n | [M-0083](../roadmap/phase-9/M-0083.md) | 📝 open |
| R-26 | **Viewpoint editor** (Tab) in the driver's view: camera height, X/Y/Z position and FOV, saved per vehicle and loaded as that view's default | camera | [M-0084](../roadmap/phase-4/M-0084.md) | ✅ done |
| R-27 | **Research reflections** in mirrors and glass, and in future wet surfaces | rendering | [M-0085](../roadmap/phase-5/M-0085.md) (branch `research/reflections`) | 🔬 started |
| R-28 | The start-point **dropdown cannot be scrolled**, and is not searchable | UI | [M-0086](../roadmap/phase-9/M-0086.md) | ✅ scroll fixed on `fix/dropdown-scroll`; search field added on `feat/combobox-search` (menus with more than 8 options) |
| R-29 | Next to *Scan now* show **when the last scan was** and a green / yellow / red status saying whether a manual scan is recommended | settings | [M-0086](../roadmap/phase-9/M-0086.md) | ✅ done on `feat/combobox-search`: status chip (icon + word + reason), rules in M-0086 |

## Suggestion (2026-10-13, fifth message)

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-30 | A **debug mode** (setting or command line, usable by agents through the protocol) in which OPTS tracks and logs **which mesh is located where on which tile** | tooling / content | [M-0087](../roadmap/phase-5/M-0087.md) | ✅ done (scene trace, [design](../architecture/scene-trace.md)) |

## Follow-up findings (2026-10-13, sixth message)

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-31 | **Freeze when opening the content section of the settings**; any synchronous blocking task must run in another thread | UI / performance | [M-0088](../roadmap/phase-9/M-0088.md) | ✅ fixed (background facts, "Calculating…" placeholders; worst frame 213-350 ms -> 17 ms) |
| R-32 | **Tile streaming** performs poorly, with noticeable freezes; investigate routes for improvement | performance | [M-0089](../roadmap/phase-4/M-0089.md) | 🟡 in progress 2026-10-18 |
| R-33 | **Default position per view**, reset with **Shift + Space** | camera | [M-0084](../roadmap/phase-4/M-0084.md) (addendum) | 📝 open |
| R-34 | A few **pedestrians have scrambled meshes** | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-35 | **Hamburg maps:** loading objects fails partially, leaving absent or scrambled objects | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 fixed 2026-10-18 (M-0066); awaiting your check |
| R-36 | **Objects take quite a bit of time to load** | performance | [M-0090](../roadmap/phase-10/M-0090.md), [M-0089](../roadmap/phase-4/M-0089.md) | 🟡 loading screen fixed (M-0089); cold model load open |
| R-37 | Many **parked-car scenery objects are partially transparent** and show the interior | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 fixed (MI-0046/0047/0053/0056); awaiting your check |
| R-38 | **Driving cars have no wheel animations** | content / traffic | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 implemented 2026-10-11, verified 2026-10-18; awaiting your check |
| R-39 | **Driving cars on Wuppertal maps are transparent** | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-40 | **Shadows** (probably overall graphics) need to look more realistic | rendering | [M-0091](../roadmap/phase-8/M-0091.md) | 📝 open |
| R-41 | **Schwebebahn stations** on the Wuppertal map do not load entirely | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 re-checked 2026-10-18, no defect found; awaiting your check |
| R-42 | On Wuppertal some **T-crossings are flipped by 90°** | content | [M-0090](../roadmap/phase-10/M-0090.md) | 🟡 not reproduced; I need a position (map, junction) |
| R-43 | M-0071 and directly related items: implemented on a branch by an agent that merges it and removes its worktree itself | UI / process | [M-0071](../roadmap/phase-9/M-0071.md) | ✅ done 2026-10-13 |

## Notes

- **R-01, R-09** overlap with the running mesh audit (P-1014, branch `agent/mesh-audit`); its registry feeds M-0066.
- **R-02**: the `.water` format is only partly specified (`formats/map/tiles.md`; four level values, semantics open). Format work comes first (docs first, [ADR-0002](../adr/0002-clean-room-policy.md)).
- **R-10**: the control protocol already lists `settings.list/get/set/reset` ([reference](../reference/control-protocol.md)). Which setting or call failed for the reporter must be reproduced first; M-0072 then closes the real gap (confirm/revert prompts, batch set, coverage test).
- **R-03** needs a decision about the HUD and the camera without a vehicle (free camera, no instruments), recorded in M-0068.
