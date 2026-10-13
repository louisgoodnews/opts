# Session Save

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-13), live verification in the [progress file](../dev/progress/session-save.md) |
| **Milestone** | [M-0069](../roadmap/phase-10/M-0069.md) (field report R-05) |
| **Decision** | [ADR-0011](../adr/0011-session-save-format.md) |
| **Code** | `crates/opts-game/src/save/` (`container.rs`, `store.rs`, `registry.rs`, `sections.rs`, `ops.rs`, `restore.rs`, `quick.rs`, `fingerprint.rs`, `ui/`), `session_vehicles/state.rs`, `control_save.rs` |
| **Tests** | `save::*::tests`, `tests/save_tests.rs`, `tests/control_protocol.rs`, `tests/fixtures/saves/` |

A save lets the user stop a drive and continue it later. The format grows by **sections** so weather, timetables, passengers, AI traffic and more can join without rewriting the save system or breaking old saves.

## 1. The container

One UTF-8 JSON file, pretty-printed (human-inspectable, diffable, attachable to a bug report):

```json
{
  "magic": "OPTS-SAVE",
  "format_version": 1,
  "opts_version": "0.1.0",
  "meta": { "name", "created_unix", "saved_unix", "play_time_s", "map", "vehicles": [..], "thumbnail"? },
  "fingerprint": { "map": ["Berlin-Spandau", "<hash>"], "vehicles": { "<file>.bus": "<hash>" } },
  "sections": { "<id>": { "version": 2, "data": { ... } } }
}
```

* `magic` and `format_version` are checked **before** anything else is read: a file from a newer build is refused with a clear message even if its layout changed; other JSON is "not a save".
* `meta` is what the save list shows without reading any section. `thumbnail` is reserved (absent today).
* `fingerprint` hashes the content the save refers to (FNV-1a 64: the map's `global.cfg` and tile list, each vehicle's `.bus` file). A mismatch is a **warning** (content changed since saving), never a refusal.
* `sections` is a map from id to `{version, data}`. The container never looks into `data`.

**Encoding.** JSON, no compression: saves are a few kilobytes to tens of kilobytes. A future large section (AI traffic) can compress its own `data`. **Limits:** a file over 16 MiB is refused when read and when written, a section over 4 MiB is refused when written, names are cut to 60 characters.

## 2. Sections and the registry

Feature code registers a `SaveSection` (`App::register_save_section`, like the settings and HUD registries):

| Field | Meaning |
| :--- | :--- |
| `id`, `version` | the key and the version `capture` writes and `apply` reads |
| `capture(&mut World) -> Result<Option<Value>, String>` | session to JSON; `None` leaves the section out |
| `prepare(&mut World, &Value)` | optional; runs **before** the session starts and writes the choices the loading pipeline reads (map, start point, date, weather) |
| `apply(&mut World, &Value)` | optional; JSON to the running session, in a `stage` |
| `migrate(from, data)` | older version to the current one |
| `check(&Value, &CheckEnv) -> Vec<Issue>` | optional; reports missing or changed content (for the list's warning and the load's refusal) |
| `stage`, `order` | `Stage::World` (world and HUD are up, before the saved vehicles are rebuilt) or `Stage::Settled` (all vehicles are back); `order` sorts within a stage |
| `required` | a section whose failure stops a save or load (`session`) |

The save system (`container`, `store`, `registry`, `ops`, `restore`) never names a section. `tests/save_tests.rs::a_section_registered_in_a_test_needs_no_change_to_the_save_system` registers a section from inside a test and runs it through capture, file, restore.

### Compatibility rules

| Case | Behaviour |
| :--- | :--- |
| Section in the file, **unknown** to this build | kept in memory, ignored, listed in `session.saves.unknown_sections`; **copied into the file when the save is overwritten** (`carry_over_foreign`) so an older build never destroys a newer build's data |
| Section **missing** from the file | skipped: the feature keeps its defaults |
| Section **older** than registered | migrated by its `migrate` (one function, any older version); no migration or a failing one: skipped with a warning |
| Section **newer** than registered | treated like unknown (kept, ignored, warning) |
| `format_version` newer | the whole file is refused (*newer format* error) |

## 3. Files, atomic writes, backups

Folder: `<user data dir>/saves/` (`~/.local/share/opts/saves` on Linux). A save has a stable **id** (a slug of its first name, `[a-z0-9_-]`, never taken from the outside without validation); renaming changes `meta.name` only.

```
<id>.save.json        the save
<id>.save.json.bak    the previous version (rotated on every write)
<id>.save.json.tmp    only while a write runs
```

A write: (1) full text to `.tmp`, `fsync`; (2) old main file to `.bak`; (3) `.tmp` renamed over the main file. The main file is never opened for writing, so a crash leaves either the old save, or (between 2 and 3) only the backup. Reading uses the **backup when the main file is missing or damaged** (the list says so); a file from a newer build is *not* replaced by the backup. Damaged files are listed with their error and can be deleted. Stale `.tmp` files are ignored and removed by the next write.

## 4. What a save holds

| Section | Holds | Restored by |
| :--- | :--- | :--- |
| `session` | map folder and name, start point, mode (`drive`), play time | `prepare`: `NewGameSetup`, `PlayClock` |
| `time` | date, clock base (start hour and minute), simulated seconds since the start | `prepare` (date, base); `apply` sets the session clock (the sky and instruments follow it) |
| `weather` | weather preset | `prepare` |
| `session_vehicles` (v2) | the registry (see [session vehicles](session-vehicles.md) §7) with, per vehicle: absolute pose, heading, velocity, yaw rate, steering, throttle, brake, retarder, parking brake, gear, rpm, **every non-zero script variable** (doors, lights, switches, destination, counters), and the `.bus` path relative to the installation | `apply` (World): the records come back as spawn requests; the spawner rebuilds each vehicle **through the normal spawn path** at its entry point, then `session_vehicles::state::apply_pending` puts the state on it once its scripts have run `{init}` and no vehicle is still spawning |
| `active_vehicle` | which vehicle the driver is in | `Settled`: `ops::switch_to` |
| `camera` | mode, previous mode, sub-camera, free camera pose (absolute), orbit and look offsets | `Settled` (falls back to the free camera when the saved mode is not offered) |
| `counters` | odometer, session duration, distance, top speed | `Settled`; the odometer continues across the activation restart (`RemoteControls::pending_odometer`) |
| `simulation` | paused, time scale | `Settled`, last. The pause overlay's own pause is not counted as the player's pause |

Version history: `session_vehicles` v1 held the registry only; v2 adds `saved_state` and `bus_rel` per record. The v1 to v2 migration changes the version number (both fields default to absent); the fixture `tests/fixtures/saves/v1-vehicles.save.json` tests it. The `viewpoint` section joins when the viewpoint store lands on master; nothing in the save system changes for it.

### Not restored yet (said so in the save list's note)

* **AI traffic, parked cars, pedestrians and trams** are not saved. They are re-seeded from the restored map and clock (they start anew at the saved place and time, so the cars around the player differ from before the save).
* **Trailer and articulation internals** are not saved: the trailer chain re-seeds behind the restored towing vehicle.
* **Audio, passengers, timetable progress, announcements** do not exist yet.
* **Script state beyond variables** (a script's internal random generator, the elapsed-time counter) restarts; scripts that keep their state in variables (all of the OMSI scripts examined so far) continue.
* **Held keys and remote controls** are released.

### Tolerances (per field)

Positions, headings and velocities are stored as JSON decimals and come back to `f32` precision (the unit tests compare exactly). A restored vehicle is placed, then the physics continues from that state, so what a player sees after the first fixed step is: pose within centimetres, speed within 0.1 km/h, the ground height re-derived from the loaded tiles (the saved height is used until they are there). Script variables are exact; scripts react to them on their next frame. The simulated clock is exact to the saved second; the sky follows it. The odometer and the session counters are exact. The AI traffic is not (see above).

## 5. Restore

```
begin_restore(id)
  ├─ read file (backup fallback), plan_restore (migrate, unknown, missing)
  ├─ required sections present? preflight: map missing, or every vehicle missing => refuse (RestoreError::Blocked)
  ├─ prepare functions: NewGameSetup (map, start, date, weather, no_vehicle = true), play clock
  └─ NextState(InGame)  (from Home: a normal start; from a drive: the same state is left and entered again)
the normal loading screen (map, sky, camera over the start point)
  └─ HUD up: Stage::World sections  ->  vehicles queued
       └─ spawner builds each vehicle  ->  state applied after {init}
            └─ nothing spawning, nothing waiting: Stage::Settled sections, toast "Loaded …", LastRestore = Done
```

Sections that fail to apply are logged and counted as warnings (toast "Loaded with N problems"); only a `required` section failing in `prepare` refuses the load. A restore that never settles (a vehicle stuck) gives up after 120 s and settles without it. A failed loading screen ends the restore (`LastRestore = Failed`).

Content problems: a missing **map**, or **every** saved vehicle missing, refuses the load with the error template (title: what happened; body: why and the names; buttons *Choose another* and *Back*). Some vehicles missing loads the rest and lists the missing ones in the warnings.

## 6. User interface

* **Pause overlay** gets *Save game…* and *Load game…*: a modal list (the pause overlay steps aside and returns). *Save game* has a name field and *Save*; an existing name asks *Overwrite this save?*; each row has *Load*, *Overwrite*, *Rename*, *Delete*. Loading over a running drive asks first.
* **Home → Load game** (the not-yet entry is gone) opens the same list in load mode.
* Rows show name, map, vehicles, play time, saved date, and warnings (map or vehicle not installed, content changed, damaged file read from its backup). The overlay's note says what is not saved yet.
* **F5** quick save, **F9** quick load (fixed slot `quicksave`; both in the Help table and the [input gating audit](reporting-feature.md)), and the setting **Gameplay → Autosave** (`gameplay.autosave_minutes`, 0 = off) writes the slot `autosave`. Slot names come from the message tables.
* Everything is localised (`save-*`, `saves-*`, `load-*` keys in `shell/l10n/`).

## 7. Control protocol (1.3.0)

`session.save {name, overwrite?}`, `session.load {name}`, `session.saves`, `session.delete_save {name}`, `session.rename_save {name, new_name}`; see the [reference](../reference/control-protocol.md). `session.load` is asynchronous: poll `session.saves` for `restore.state` (`loading`, `done`, `failed`).

## 8. Determinism

The [determinism and replay design](determinism-replays.md) is not written yet. A save is a *state snapshot*, not a replay: restoring does not reproduce the run that followed, only its starting state (minus the unsaved parts of §4). The fixed-step simulation continues from the restored vehicle state; nothing in the save depends on frame timing.
