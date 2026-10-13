# Progress: M-0068 session vehicles + M-0075 place label + M-0076 loading vehicle

Branch `agent/session-vehicles`, worktree `/home/louisgoodnews/Dokumente/opts-wt-session-vehicles` (from master d594a9e). Never touch `/home/louisgoodnews/Dokumente/opts`.
Live runs: throwaway XDG under `~/.local/share/opts/benchmark/session-vehicles/` (delete afterwards). OMSI: `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`.

## Resume
`git status --short; git log --oneline master..HEAD; cargo check --workspace --all-targets --all-features`
Gates: fmt, clippy -D warnings, `cargo test --workspace --all-features`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `python3 scripts/check_links.py`, `python3 scripts/gen_roadmap_copy.py --check`.

## Design
- `crates/opts-game/src/session_vehicles/{registry,ops,mod}.rs`: `SessionVehicles` registry (serde = save section `session_vehicles`), `SessionVehicle(id)` component on roots, active vehicle = the one with `PlayerVehicle`; ops switch/despawn/respawn/sync_lost on `&mut World`; `PlaceLabel`; F8 quick switch (Shift = previous).
- Spawn requests are `Spawning` records; `omsi_world/respawn.rs::handle_vehicle_requests` works them down, one in flight, then `switch_to`.
- `load_player_vehicle(.., session_id, activate) -> SpawnedVehicle`.
- Map-only: `NewGameSetup.no_vehicle`, `WorldLoadJob.no_vehicle`, wizard list entry `:none`.
- opts-bevy: `DriverHead` + `sync_player_head` keep exactly one `PlayerHead`.

## Status
All code, tests, docs done and committed on the branch; live verification done (map-only start, two buses via session.spawn, switch, 85 m drive with the place label vanishing at ~25 m, despawn inactive and active, pause menu rows, F8, despawn dialog). Scratch deleted.
Remaining: final gates (fmt, clippy of own crates, workspace tests, rustdoc, links, roadmap copy) and the report.
Pre-existing, not from this branch: clippy errors in `opts-core` examples and `tests/uv_decoder.rs`; `opts-benchmark --test run_control crash_hang_recycle_and_resume` (checked separately).
