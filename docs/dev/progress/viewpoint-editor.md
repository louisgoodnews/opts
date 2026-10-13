# Progress: M-0084 viewpoint editor (branch feat/viewpoint-editor)

Worktree: /home/louisgoodnews/Dokumente/opts-wt-viewpoint. Resume from this file.

Finding: the driver's view is **key 2 (Cockpit)**, not 4 (4 = Passenger). The label reads `Driver's view x/n`
(sub camera index / `[add_camera_driver]` count). The editor therefore works in mode 2.

Design: offsets (X right, Y up, Z forward, metres, limit +-0.6) and horizontal FOV (30-120) per
(vehicle key = `.bus` file stem lowercase, driver camera index), applied on top of the vehicle preset.

## Batches
- [ ] A: opts-bevy `camera/viewpoint.rs` (model, state, keys, camera apply, gating)
- [ ] B: opts-game store (`viewpoints.rs`, JSON v1 next to drive_history.json), load on spawn/enter, persist
- [ ] C: panel UI (HUD style), l10n de/en, Help row, gating audit
- [ ] D: control protocol camera.viewpoint.get/set/save/reset + reference doc
- [ ] E: tests
- [ ] F: docs (camera-system, M-0084, ROADMAP, BOARD)
- [ ] G: live verification, gates
