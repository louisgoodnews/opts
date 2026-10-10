# 🎛️ Control Protocol & Hot Reload — Final Report (2026-10-10)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0043](../roadmap/phase-5/M-0043.md) |
| **Design** | [Control Protocol](../architecture/control-protocol.md), [Hot Reload](../architecture/hot-reload.md), [ADR-0006](../adr/0006-control-protocol.md), [ADR-0007](../adr/0007-hot-reload.md) |
| **Commits** | `49f5513` design · `367dcf7` client · `c08b504` server · `a01294f` hot reload · `0730194` docs · Part F (this report) |
| **Verification** | `cargo fmt`, `clippy -D warnings`, 421 tests, link check — all green |

## Result

A running OPTS can be observed and driven from a terminal or by an agent, and edits to OMSI files take
effect without a restart. Verified on BRT Berlin and X10 Berlin with the real game window.

## Commands

The protocol table has **49** methods; **36** are implemented (including `auth`, handled by the server layer):

| Group | Implemented | Deferred (answer `1007`) |
| :--- | :--- | :--- |
| meta | `ping`, `version`, `help`, `capabilities`, `auth`, `shutdown` | — |
| query | `state`, `scene`, `camera.get`, `sim.get`, `log.tail`, `log.filter` | `entity.list`, `entity.get`, `resource.list`, `resource.get`, `asset.list`, `asset.get` |
| input | `input.key`, `key_hold`, `key_release`, `text`, `mouse.move`, `mouse.click`, `mouse.scroll` | — |
| sim | `sim.pause`, `resume`, `step`, `time_scale` | `sim.set_time` |
| scene | `scene.load_map`, `scene.unload`, `camera.set_mode`, `camera.teleport`, `vehicle.set` | `entity.spawn`, `entity.despawn`, `entity.set_component` |
| reload | `reload.asset`, `reload.config`, `reload.scene`, `watch.add`, `watch.remove`, `watch.list` | — |
| debug | — | `debug.wireframe`, `debug.normals`, `debug.stats` |
| output | `screenshot`, `report` | — |
| batch | `batch` | — |

**Why deferred:** `entity.*`, `resource.*` and `asset.*` need components to derive `Reflect` or a newer Bevy
(`bevy_remote`, see M-0044); `debug.*` needs render-pipeline hooks; `sim.set_time` needs a session clock the game does not
have yet. Event streaming (`opts-cli watch`) is not part of protocol 1.0.

## Hot reload paths

| File | Works | Evidence |
| :--- | :--- | :--- |
| `.o3d` / `.x` meshes | ✅ tiles that use the mesh are rebuilt | live: 2 tiles + 1 mesh re-read |
| `.sco` | ✅ | live: 5 tiles rebuilt |
| `.sli` | ✅ | live: 2 tiles rebuilt (`str_2spur_8m_altonaer1.sli`) |
| `.map` / `.terrain` | ✅ | live: 1 tile rebuilt |
| textures | ✅ in place (code, unit-tested); the live check touched a file the game had not loaded | integration test |
| `.bus`, vehicle model config, vehicle meshes | ✅ session restart (vehicle returns to the start position) | live |
| `.cfg` other than `model*`/`global.cfg` | validated for readability only | by design |
| `.wav` | classified, nothing to rebuild yet | M-0027 |

## Tests

* `opts-cli`: 20 unit tests (protocol, parameter parsing, output, REPL helpers) and 8 mock-server tests
  (calls, error data, refused connection → exit 2, garbage reply → exit 3, batch, reconnect after restart, token, exit codes).
* `opts-bevy/tests/control.rs` (10): real socket inside a headless app — meta, error codes, key press/release, pause/step/scale,
  batches (incl. `stop_on_error`), token enforcement, malformed JSON, log ring, shutdown.
* `opts-bevy/tests/hot_reload.rs` (5): real file watcher — one reload per burst of writes, broken save fails and the next
  good one recovers, scratch/unknown files ignored, `reload.*` replies, `watch.*`.
* Unit tests for debounce, classification, validation, path policy (`..` and symlinks), key names, the log ring, the server limiter.

## Manual runs (all with the real game)

**F1 protocol test** (`--cli-port 44201`): `ping`, `state`, `camera.set_mode free`, `input.key W --duration 2000`, `sim.pause`,
`sim.step 60` (answered after 60 frames, time stopped again), `screenshot --path` into an allowed directory (1.1 MB PNG written),
`report` (PNG + `.report` containing the text). All exit code 0.

**F2 hot reload:** `touch` on a tile, a spline profile in use, a mesh in use, a scenery object in use → each logged
`reload requested` then `reload applied … N tile(s) rebuilt`; entity and tile counts unchanged afterwards. (OMSI files were only
touched, never edited.)

**F3 broken file** (a temporary directory added with `watch.add`): a valid save → `reload applied`; truncating it →
`WARN reload failed, keeping the previous content: the file is empty`; the app stayed in `InGame`. `reload.asset` on a broken file
answers `1008` with the parser message. The unit/integration tests cover truncated meshes, textures and configs.

**F4 agent simulation:** `crates/opts-cli/examples/agent_session.sh` ran end to end (load map, set vehicle, camera, drive 5 s,
pause, screenshot, `jq` over `scene` and warnings), exit 0. The Rust examples `drive_and_screenshot`, `reload_iterate` and
`scene_probe` also ran successfully.

## Sample session

```text
$ opts-cli scene.load_map "…/maps/BRT Berlin"      → map: BRT Berlin, tiles_loaded: 8, vehicle: MAN SD200 - SD77   (≈ 2 s)
$ opts-cli camera.set_mode free                    → mode: Free
$ opts-cli camera.teleport 3910 3580 5 --yaw 90    → translation: [3910, 5, 3580]
$ opts-cli sim.pause && opts-cli sim.step 60       → stepped: 60
$ opts-cli screenshot                              → path: ~/.local/share/opts/reports/screenshot_….png
$ opts-cli reload.asset "…/Sceneryobjects/Trees_MC/tree_medium_01.sco" → 5 tile(s) rebuilt
$ opts-cli log.filter --level warn --n 3           → the last warnings, one per line
```

## Limitations and observations

* **Vehicle reload loses driving state** (the session restarts at the start position).
* **Reload granularity is the tile**, via dependency sets; editing a texture used inside map masks rebuilds tiles.
* **Introspection is shallow** until M-0044 or `Reflect` derives (see above).
* **`scene.load_map` can stall behind a catalogue rescan:** the loading stage waits while the catalogue is being rebuilt, and the
  command then fails with `1005` after 120 s. This is existing menu behaviour, surfaced by the protocol; the `opts-cli` default
  timeout for slow commands is 120 s.
* **Closing the game window ends the app** (observed once during testing): the client then reports "the application dropped the
  request" (`-32603`, exit 2/1 depending on the moment).
* Texture names consisting of dashes (`-----`, "no texture" in several `.sco` files) no longer log an `ERROR`.
* Protocol facts that differ from the first design draft: camera modes use the number keys (not F-keys), `camera.teleport` takes
  absolute OMSI coordinates, the table has 49 methods (not 47).

## Follow-ups (not scheduled)

`entity.*`/`resource.*` via reflection or `bevy_remote`, `debug.*`, event streaming for `watch`, vehicle-state snapshot on reload,
a headless key-event test for the F1 toggle (audit finding F-026 can now use the control protocol).

**Clean-room attestation (ADR-0002):** no OMSI binaries were inspected; no OMSI asset was committed.
