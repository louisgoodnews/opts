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
Done (uncommitted until first WIP commit, not yet compiled): registry, ops, plugin, spawner rewrite, loading pipeline registration, wizard none entry, l10n keys.
TODO: compile fixes; InGamePlugin adds SessionVehiclesPlugin + `pub mod session_vehicles` in lib.rs; ground_height/collision use SessionVehicle + per-entity BodyMotion; HUD (place label in session widget, no instruments without vehicle, rebuild on kind change, hud.state place_label, source test no map literals); M-0076 loading line (InGameSessionInfo.vehicle_name/none set once in setup_in_game_session, LoadingUi uses it, test first frame x3+none, source test no default); vehicle menu list/switch/despawn(dialog)/respawn; control protocol session.* (+docs regen, protocol 1.2.0); headless tests (vehicle-less N frames, per command); live verification; docs (M-0068/75/76, READMEs, ROADMAP, BOARD, field report, architecture/session-vehicles.md); gates.
