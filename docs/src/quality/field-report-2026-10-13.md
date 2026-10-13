# Field Report 2026-10-13 (Project Lead)

| Metadata | Value |
| :--- | :--- |
| **Source** | project lead, from hands-on testing of the merged build (`master` at `99a25d4`) |
| **Date** | 2026-10-13 |
| **Status** | 📝 Intake complete; every item has a milestone. Reproduction and root-cause notes are added by the milestone's owner. |

Each reported item gets an ID (R-nn), is traced to a milestone, and is closed only when the milestone's acceptance criteria are met. Wording is the project lead's, shortened.

| ID | Report | Area | Milestone | Status |
| :--- | :--- | :--- | :--- | :--- |
| R-01 | Some **parked car** meshes are not loaded correctly: scrambled, missing or transparent meshes | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) (discovery: P-1014 mesh audit) | 📝 open |
| R-02 | OPTS **cannot load water**, although `.water` files lie next to the tiles; they need to be handled | world | [M-0067](../roadmap/phase-10/M-0067.md) | 📝 open |
| R-03 | The simulation must be **startable without a bus**: the vehicle selection needs a way to load only the map | session / UI | [M-0068](../roadmap/phase-10/M-0068.md) | 📝 open |
| R-04 | OPTS must **track the user vehicles spawned in a session**, let the user **change the active bus** and **respawn any** previously spawned, still existing bus, not only the active one | session | [M-0068](../roadmap/phase-10/M-0068.md) | 📝 open |
| R-05 | **Session persistence**: save the current session and restore it; expandable for weather, timetables and more | session | [M-0069](../roadmap/phase-10/M-0069.md) | 📝 open |
| R-06 | **Adhere to the roadmap** again and **expand it** | process | [M-0074](../roadmap/phase-5/M-0074.md) | 📝 open |
| R-07 | The date/time step of a new game must **default to the map's defined time**, otherwise the **current date and time** | UI / wizard | [M-0070](../roadmap/phase-9/M-0070.md) | 📝 open |
| R-08 | **HUD rework**: control information (hints, speed, gear, parking brake) in the **bottom-left**, a **mini map bottom-right**, in line with the UI standards | UI / HUD | [M-0071](../roadmap/phase-9/M-0071.md) | 📝 open |
| R-09 | The **BRT Citaro** has UV/mesh issues | content / meshes | [M-0066](../roadmap/phase-10/M-0066.md) | 📝 open |
| R-10 | The **control protocol** must allow AI agents to **set settings via the CLI** | tooling | [M-0072](../roadmap/phase-5/M-0072.md) (protocol 1.1.0 has `settings.*`; the exact gap is to be reproduced) | 📝 open |
| R-11 | **Suspension, brakes and more**: every vehicle defines its own mass, axles etc., which must be respected; OPTS provides a world physics system, the vehicles their individual behaviour | simulation | [M-0073](../roadmap/phase-10/M-0073.md) | 📝 open |

## Notes

- **R-01, R-09** overlap with the running mesh audit (P-1014, branch `agent/mesh-audit`); its registry feeds M-0066.
- **R-02**: the `.water` format is only partly specified (`formats/map/tiles.md`; four level values, semantics open). Format work comes first (docs first, [ADR-0002](../adr/0002-clean-room-policy.md)).
- **R-10**: the control protocol already lists `settings.list/get/set/reset` ([reference](../reference/control-protocol.md)). Which setting or call failed for the reporter must be reproduced first; M-0072 then closes the real gap (confirm/revert prompts, batch set, coverage test).
- **R-03** needs a decision about the HUD and the camera without a vehicle (free camera, no instruments), recorded in M-0068.
