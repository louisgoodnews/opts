# Session Vehicles

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-13) |
| **Milestones** | [M-0068](../roadmap/phase-10/M-0068.md) (registry, switch, despawn, map-only start), [M-0075](../roadmap/phase-9/M-0075.md) (HUD place label), [M-0076](../roadmap/phase-9/M-0076.md) (loading line) |
| **Code** | `crates/opts-game/src/session_vehicles/` (`registry.rs`, `ops.rs`, `mod.rs`), `in_game/omsi_world/respawn.rs` (spawner), `control_session.rs` (protocol) |
| **Tests** | `session_vehicles::tests`, `tests/control_protocol.rs`, `tests/loading_tests.rs`, `tests/wizard_tests.rs`, `hud::tests` |

A session is a map plus **zero or more user vehicles**. A *user vehicle* is one the user brought in: the one chosen in the wizard, one spawned later from *Pause → Vehicle*, or one spawned through the [control protocol](control-protocol.md). Parked cars and AI traffic are not session vehicles.

## 1. The registry

`SessionVehicles` is a resource: a list of `SessionVehicleRecord`s in spawn order. Every record has

| Field | Meaning |
| :--- | :--- |
| `id` | stable session id (`u32`, 1, 2, ...; never reused in a session) |
| `catalogue_id`, `name`, `bus_file` | what it is; the display name is manufacturer and model |
| `repaint`, `setvars` | the repaint and the equipment options it was built with |
| `entry` | the entry point that was asked for: the session's start entry, a record number or a name |
| `spawn_entry_id`, `spawn_entry_name` | the `[entrypoints]` record it was really placed at (`global.cfg`, see [map format](../formats/map/global-cfg.md)) and its name |
| `spawn_point` | absolute OMSI position (m) and compass heading (degrees) |
| `lifecycle` | `spawning`, `alive`, `despawned`, `failed` (+ `despawn_reason`: `user`, `lost`, `cancelled`; `error`) |
| *runtime only* | `entity`, `trailers`, `camera`, `place_dismissed` (not saved) |

The component `SessionVehicle(id)` on the root entity links back; the **active** vehicle additionally carries `opts_bevy::PlayerVehicle`, which is what every input, camera, HUD and script system already queries. There is exactly one active vehicle or none.

### Lifecycle

```
request_spawn ──► Spawning ──► (spawner: entry resolved, meshes decoded, built) ──► Alive ──► Despawned
                      │                                                              │
                      └── Failed (error kept)                       entity vanished ──┘ (reason: lost, warning toast)
```

* **Spawning** is also the spawn queue: the spawner (`handle_vehicle_requests`) takes the oldest request, resolves its entry point to a place, waits for the scrambled meshes to decode in the background, builds the vehicle and then makes it the active one. One request is in flight at a time; the rest wait.
* The session's first vehicle (the wizard's choice) is built by the loading pipeline and registered as `Alive` and active at once. With *Drive without a vehicle* the registry stays empty.
* Entering or leaving a session clears the registry; the despawned entities go with the session's `InGameEntity` teardown.

## 2. Map-only start

*Plan a drive → What* offers **Drive without a vehicle** twice: as the first entry of the list and as a button in the detail of any vehicle. The choice is `NewGameSetup.no_vehicle`; it is remembered in the drive history. The session then

* loads the map and the start tile as usual, but builds no vehicle (`WorldLoadJob.no_vehicle`); the camera is the free camera over the chosen entry point;
* runs the simulation (time, sky, traffic, scenery scripts) normally;
* shows no instruments: the HUD hides the instrument slot while `HudValues.has_vehicle` is false; the status bar, hints and the pause button stay;
* has no place label, and the loading screen's vehicle tile says *No vehicle*.

An explicit `--vehicle-path` wins over the choice. Systems that read the player vehicle use `Query::single`/`Option` and do nothing without one; `loading_tests::a_session_without_a_vehicle_loads_and_runs_with_a_free_camera` runs 360 frames without one.

## 3. Switch, despawn, respawn

All three are functions on the `World` in `session_vehicles::ops`, so the pause menu, the key and the control protocol run the same code.

**Switch** (`switch_to`, key **F8**, Shift+F8 for the previous one; the key is in the Help table): the previous active vehicle is *parked*: throttle, brake and steering released, **parking brake on**; it keeps its state and the physics brings it to rest. The new vehicle receives `PlayerVehicle`; the camera sub-camera resets to the vehicle's standard driver camera; held remote controls are released; `sync_player_head` moves `PlayerHead` (the ego/cockpit eye) to the new driver; the HUD is rebuilt when the kind of vehicle changes (bus, articulated bus, tram); the script engine focus follows because triggers and key bindings go to `PlayerVehicle`. Only the active vehicle gets driver input (`vehicle_input_system` filters `With<PlayerVehicle>`; the input-gating audit stays green).

**Despawn** (`despawn`): removes the root, the trailer chain (`ArticulatedChain`), the composition parts (`VehicleComposition`) and, recursively, every child (`InGameEntity`). Any vehicle can be despawned, active or not. Despawning the **active** one makes the *most recently spawned other living vehicle* active; with none left the camera becomes free where it is (the HUD loses its instruments). A request that is still `Spawning` is cancelled. In the pause menu the button asks first (the shell's confirmation dialog `session.despawn`, safe choice first); the control protocol does not ask.

**Respawn** (`respawn`): puts a living vehicle back at its recorded spawn point at rest, pedals released; the articulation system treats the position jump as a placement and re-seeds the trailers. It also shows the place label again.

**Reset on road** (`reset`, `session.reset_road`; `session_vehicles/road_reset.rs`): puts the vehicle on the nearest road lane within 60 m of where it stands (a lane that runs its way, within 45°, unless another lane is more than 6 m nearer), faces it along the lane (as it faced, if it stood against it), at rest. For a bus that was turned, pushed off the road or got stuck; the spawn point stays as it was.

## 4. Place label (HUD)

The `session` widget of the status bar shows the map and, while it applies, the **name of the entry point the active vehicle was spawned at** (`PlaceLabel`, computed from the registry; no place name exists in the HUD sources, a source test forbids them). The label is shown while the vehicle stands within **25 m** (`PLACE_LABEL_HIDE_DISTANCE_M`) of its spawn point. Once it has been further away it stays hidden (also when it returns) until the vehicle is respawned or a new vehicle is spawned. Despawned or no vehicle: no label. `hud.state` and `session.vehicles` expose it as `place_label`.

If the start did not resolve an entry point (the world fell back to the first road spline) the name is empty and no label shows.

## 5. Loading screen

The vehicle tile of the loading screen is decided **once**, when the session is entered (`setup_in_game_session`), from the wizard's choice and the catalogue: the vehicle's display name and repaint, or *No vehicle*. Nothing overwrites it during the load (it used to switch from the setup key to the loaded manifest's name at about 70 %). The defaults of `NewGameSetup` no longer name a vehicle, entry point or map-specific string.

## 6. Streaming and destruction

Session vehicles are root entities, **not** children of map tiles, so tile streaming never removes them. Should an entity vanish anyway, `ops::sync_lost` notices it each frame: the record becomes `Despawned(lost)`, its trailer parts are removed, the next vehicle becomes active and a warning toast names the vehicle. No record points at a missing entity and no part of a vehicle is left behind (`tests::a_vehicle_that_vanishes_is_noticed_and_never_dangles`).

Known limits: a parked vehicle outside the streamed tiles has no ground under it (it stays at its height and, with the parking brake on, at its place); vehicles do not collide with each other yet (only with the static world and traffic); the `[texttexture]` definitions of the vehicle model are kept for the vehicle built last.

## 7. Save system (M-0069)

`SessionVehicles` is plain data and serialises as one section, `session_vehicles` (version 1): `{version, next_id, active, vehicles:[records]}` with the alive and spawning records (despawned ones are history). `SessionVehicles::to_section` and `from_section` are tested for a round trip; on restore the records come back as `Spawning` requests at their saved `spawn_point` with their saved repaint and options, and the saved `active` id is kept so the spawner can restore it. The vehicle's *current* pose and state are the save system's to add as further fields of the record.

## 8. Control protocol

`session.vehicles`, `session.spawn`, `session.switch`, `session.despawn`, `session.respawn` (protocol 1.2.0); see the [reference](../reference/control-protocol.md). `session.spawn` is asynchronous: it returns the id at once and the vehicle appears in `session.vehicles` as `spawning`, then `alive` or `failed`.
