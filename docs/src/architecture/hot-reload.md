# ♻️ Hot Reload

**Status:** 🟡 Draft (design for M-0043) · **Decision record:** [ADR-0007](../adr/0007-hot-reload.md) · **Code:** `opts-bevy::hot_reload`

Editing an OMSI file while the game runs updates the running scene without a restart. This page defines what
is watched, how events become reloads, and what happens when a file is broken.

## 1. Scope

| Category | Extensions | Reload behaviour |
| :--- | :--- | :--- |
| Meshes | `.o3d`, `.x` | drop the content-cache entry, re-parse, rebuild the `Mesh`; entities using it are rebuilt |
| Textures | `.dds`, `.tga`, `.bmp`, `.png`, `.jpg` | drop the texture-cache entry, decode again, replace the pixels of the existing `Image` (handles stay valid) |
| Configs | `.cfg`, `.bus`, `.sco`, `.sli`, `.hum` | re-parse; affected objects/splines/vehicle are despawned and respawned |
| Map tiles | `.map`, `.terrain` | the resident tile is despawned and re-queued |
| Audio | `.wav` | reserved (M-0027) |
| Rust sources | `.rs` | **out of scope** — needs a rebuild (a dylib-based reloader is a separate project) |

OPTS loads OMSI content through its own `ContentLoader`, not Bevy's `AssetServer`; Bevy's built-in
`file_watcher` therefore only covers the `assets/` folder (fonts) and is not used for this feature.

## 2. Watch roots

Watched by default while a session runs: the current map folder, the folders of the loaded vehicle (`.bus`,
its `model/` and `texture/` folders), and the folders of the scenery objects and spline profiles placed on
resident tiles (discovered while loading). `--hot-reload-root <PATH>` adds roots, `--no-hot-reload` disables
the watcher. At runtime `watch.add`/`watch.remove`/`watch.list` (control protocol) change the set. The watcher
is recursive, ignores editor temp files (`*~`, `*.swp`, `.#*`, `*.tmp`) and files outside the allowed roots.

## 3. Pipeline

```
notify thread ──raw events──▶ debouncer (250 ms window, per path) ──▶ ReloadEvent{path, kind}
                                                                           │ (Bevy event, once per frame)
                                  ┌─────────────────────────────────────────┴───────────────────┐
                                  ▼                                                              ▼
                       classify by extension                                         reload.* control commands
                                  ▼
            validate (parse on a task-pool thread; nothing is changed yet)
                                  ▼ ok                                  ▼ error
                    apply on the main thread                   ReloadFailed{path, error}  + WARN
```

**Debounce.** Editors write in bursts (truncate + write, rename-over, backup files). All events for a path
within the window collapse into one; the window restarts on each new event (trailing debounce) and is capped
at 2 s so a continuously written file still reloads. Configurable with `--hot-reload-debounce-ms`.

**Validate first, then apply.** The new file is parsed completely before anything is torn down. Only a
successful parse reaches the apply step, so a half-saved file can never remove working content.

## 4. Applying a reload

A reverse index `path → users` is maintained by the world loader when it spawns content (mesh path → object
entities, `.sco` → object entities, `.sli` → spline entities, texture path → materials). This turns "which
entities are affected" into a lookup.

| Kind | Apply |
| :--- | :--- |
| Mesh | `ContentCache::invalidate(path)`, then respawn the object entities that used it (their transform is kept). Entities for the vehicle are rebuilt through the vehicle path below. |
| Texture | decode, then overwrite the existing `Image` data in place; no entity changes |
| `.sco` | invalidate the parsed object, respawn every placed instance |
| `.sli` | invalidate the profile, respawn the splines (and their materials) that use it |
| `.bus` / vehicle `.cfg` | rebuild the player vehicle; the previous state (transform, velocity, gear, doors) is snapshotted and restored |
| `.map` / `.terrain` | despawn the resident tile (`TileRoot`), re-queue it in the streamer |

All respawns happen in one frame, in the `PostUpdate` schedule, after the frame's gameplay systems.

## 5. Catalogue

Installed-content changes can invalidate catalogue rows (`opts-catalogue`, the SQLite index used by the menus).
A reload does **not** rescan: the catalogue stores a content fingerprint per file, and the next menu visit or
`catalogue.rescan` notices the change. The running scene never depends on catalogue rows.

## 6. Notification

Every stage is observable: `INFO reload requested`, `INFO reload applied (path, kind, ms)`,
`WARN reload failed (path, error)` under target `opts_bevy::hot_reload`; Bevy events `ReloadEvent`,
`ReloadApplied`, `ReloadFailed` for systems; `log.tail` and `debug.stats` (reload counters) for the control
protocol. A future event stream will push them to subscribed clients.

## 7. Failure handling

A file that fails to parse (truncated, mid-edit, wrong encoding) keeps the previous asset and state: log a
`WARN` with path and error, emit `ReloadFailed`, count it in the stats. The next successful save reloads
normally. A file that disappears is treated as a failure, not as a deletion — the running scene keeps its
copy until the session ends.

## 8. Interaction with the simulation

Reloading while driving is supported. The vehicle snapshot/restore (§4) keeps the bus moving; tile
respawns do not touch the vehicle entity. Reloads are applied between simulation ticks, so physics never sees
a half-built entity. With the simulation paused (`sim.pause`) reloads still apply; a following `sim.step`
shows the result.

## 9. Path to embedded scripting

Hot-reloading a scripting language (Rhai, Lua) is a different feature: scripts hold state and run every
frame. It would reuse the watcher, debouncer and *validate-then-apply* rule of this page but needs a
sandbox, a stable engine API and a state-migration policy, and therefore its own ADR and milestone (compare
M-0026, the OMSI script VM, which already covers `.osc` files).
