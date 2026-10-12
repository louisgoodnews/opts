# Performance Profiling and Frame Pacing

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Method in place (2026-10-12); findings below |
| **Code** | `opts-logging` `layer/timing.rs`, `opts-game` feature `profile`, `app.rs` (fixed step, virtual-time limit) |

## Frame pacing

* **Rendering is uncapped.** `PresentMode::AutoNoVsync` unless `--vsync`; there is no frame limiter, the hardware sets the rate.
* **The simulation step is fixed on purpose** (default 64 Hz, `--physics-hz`). A variable step would make vehicle physics, collisions and the script VM (`Timegap`) depend on the frame rate: results would differ between machines and tunnelling through thin obstacles would depend on the frame time. Every drawn frame instead interpolates between the last two simulation states ([smooth presentation](vehicle-ecs-bridge.md)), so the screen shows motion at the display rate, not at the step rate. Raising `--physics-hz` lowers input latency and the interpolation error at the cost of CPU.
* **No catch-up spiral.** `Time<Virtual>` is limited to 50 ms per frame: after a hitch the simulation replays at most 50 ms (3 steps at 64 Hz) instead of up to 250 ms, so one slow frame no longer makes the next ones slow too.

## Measuring

1. `OPTS_DBG_FRAMES=<file>` writes one CSV line per frame (time, frame time, position, speed, fixed steps).
2. A profiling build prints the busy time of every Bevy system:

   ```
   cargo build -p opts-game --features profile --target-dir target/profile
   OPTS_SYSTEM_TIMING=15 target/profile/debug/opts-game ... 2> timing.txt
   ```

   `profile` turns on Bevy's `trace` and `debug` features (system spans with names). Every 15 s the layer prints the 40 systems with the highest summed time and then the 40 with the worst single run (frame spikes). Times are summed over threads; a `schedule:` line contains its systems. No external profiler is needed (`perf` and `gdb -p` are not available in the sandbox: `ptrace` is blocked).

## Findings, Bad Hügelsdorf with the NF6D tram (1080p, 3,727 objects, 16 k entities, 0.9 M vertices)

| Item | Per frame | Remark |
| :--- | :--- | :--- |
| Standing still | 4 ms (250 fps) | |
| Driving | 9–16 ms (60–105 fps), spikes 40–80 ms | |
| `Render` schedule | 7 ms | `render_system` 3 ms, `submit_pending_command_buffers` 1.8 ms, shadow instance buffers 0.5 ms: GPU submission of 16 k entities |
| `Main` schedule | 4 ms | `propagate_parent_transforms` 0.16 ms mean but **23 ms spikes** (a whole tile subtree spawns), `check_dir_light_mesh_visibility` 0.14 ms (7 ms spikes), text and UI layout 0.5 ms |
| OPTS systems | < 0.5 ms | `vehicle_animation_system` 0.2 ms, `vehicle_light_map_system` 0.1 ms, `spawn_traffic` 12 ms once |

The earlier "choppy tram" was therefore not the interpolation (exact, see [smooth presentation](vehicle-ecs-bridge.md)) but **hitches**: a frame that spawns a tile (thousands of entities, transform propagation of the new subtree) takes 40–80 ms, and the fixed-step catch-up after it made the next frame slower. The catch-up is limited now; the spawn spikes remain.

## Next steps

* Spread tile spawning over frames (spawn the tile root first, objects in batches of ≈ 500 entities) and despawn old tiles in batches.
* Fewer entities: merge the static meshes of a vehicle part with the same material (the NF6D cars carry ≈ 1,500 mesh children each; every moving root propagates all of them).
* Visibility ranges for small scenery (signs, lamps, bushes) beyond 150 m; shadow casters only within 80 m.
* The same layer can time any Bevy build; keep a before/after table here when one of these lands.
