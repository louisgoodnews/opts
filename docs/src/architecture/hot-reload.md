# ♻️ Hot Reload

**Status:** 🟡 Draft (design for M-0043) · **Decision record:** [ADR-0007](../adr/0007-hot-reload.md) · **Code:** `opts-bevy::hot_reload`

Editing an OMSI file while the game runs updates the running scene without a restart. This page defines what
is watched, how events become reloads, and what happens when a file is broken.

## 1. Scope

| Category | Extensions | Reload behaviour |
| :--- | :--- | :--- |
| Meshes | `.o3d`, `.x` | drop the content-cache entry, re-parse, rebuild the `Mesh`; entities using it are rebuilt |
| Textures | `.dds`, `.tga`, `.bmp`, `.png`, `.jpg` | drop the texture-cache entry, decode again, replace the pixels of the existing `Image` (handles stay valid) |
| Configs | `.cfg`, `.bus`, `.sco`, `.sli`, `.hum` | re-parse and validate (`model*.cfg` and `global.cfg` with their parsers, other `.cfg` only for readability); affected tiles/vehicle are rebuilt |
| Map tiles | `.map`, `.terrain` | the resident tile is despawned and re-queued |
| Audio | `.wav` | reserved (M-0027) |
| Rust sources | `.rs` | **out of scope** — needs a rebuild (a dylib-based reloader is a separate project) |

OPTS loads OMSI content through its own `ContentLoader`, not Bevy's `AssetServer`; Bevy's built-in
`file_watcher` therefore only covers the `assets/` folder (fonts) and is not used for this feature.

## 2. Watch roots

While a session runs the watcher covers (a) the map folder, recursively, and (b) every directory the session
has loaded content from — scenery object, spline, model and texture folders, the vehicle's folders — each
non-recursively. (b) is refreshed once a second from the loader's caches, so newly streamed content is
picked up without watching the whole installation (thousands of directories). `--hot-reload-root <DIR>`
adds recursive roots, `--no-hot-reload` disables the watcher. At runtime `watch.add`/`watch.remove`/`watch.list`
(control protocol) change the set; at most 3,000 directories are watched. Editor scratch files (`*~`, `*.swp`,
`.#*`, `*.tmp`, `*.bak`) and files of unknown type are ignored.

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

While a tile loads, the world loader records every file it was built from (the `.map` and `.terrain`
files, the `.sco`/`.sli` files, the `.o3d`/`.x` meshes and the ground masks) as the tile's *dependency set*.
A validated change is looked up in those sets, so the granularity is the **tile**.

| Kind | Apply |
| :--- | :--- |
| Texture already loaded | decode again and overwrite the existing `Image` in place; materials update without a rebuild |
| Mesh | drop the cached meshes built from the file, rebuild the tiles whose dependency set contains it |
| `.sco` / `.sli` | forget the parsed object (and the spline materials built from a profile), rebuild the tiles that use it |
| `.map` / `.terrain` | rebuild that tile |
| Map ground texture | rebuild the ground layer array and every resident tile |
| Vehicle file (`.bus`, model config, vehicle mesh) | the session restarts on the same map and vehicle — the vehicle is back at the start position |
| `reload.scene` | rebuild every resident tile and forget all parsed objects and meshes |

Rebuilding a tile despawns its root entity and puts the tile at the front of the streamer's queue; the next
frames load it again from the (partly cached) content. Everything happens in the `Update` schedule between
simulation ticks.

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

Tile rebuilds do not touch the vehicle entity, so scenery can be edited while driving. A change to a vehicle
file restarts the session; the vehicle's driving state is **not** preserved yet (a snapshot/restore of the
chassis state is a possible follow-up). With the simulation paused (`sim.pause`) reloads still apply; a
following `sim.step` shows the result.

## 9. Path to embedded scripting

Hot-reloading a scripting language (Rhai, Lua) is a different feature: scripts hold state and run every
frame. It would reuse the watcher, debouncer and *validate-then-apply* rule of this page but needs a
sandbox, a stable engine API and a state-migration policy, and therefore its own ADR and milestone (compare
M-0026, the OMSI script VM, which already covers `.osc` files).
