# Progress: M-0069 session save and restore

Branch `feat/session-save`, worktree `/home/louisgoodnews/Dokumente/opts-wt-session-save` (from master ac0bd94). Never touch `/home/louisgoodnews/Dokumente/opts`, `~/.config/opts`, `~/.cache/opts`.
Live runs: throwaway XDG under `~/.local/share/opts/benchmark/session-save/` (delete afterwards). OMSI: `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`.

## Resume
`git status --short; git log --oneline master..HEAD; cargo check --workspace --all-targets --all-features`
Gates: fmt, clippy -D warnings, `cargo test --workspace --all-features`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `python3 scripts/check_links.py`, `python3 scripts/gen_roadmap_copy.py --check`. Also `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`, `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`.

## Design (decided)
- ADR-0011 `docs/src/adr/0011-session-save-format.md`; page `docs/src/architecture/session-save.md`.
- Code: `crates/opts-game/src/save/`: `container.rs` (JSON container, magic `OPTS-SAVE`, format_version 1), `store.rs` (atomic write tmp+fsync+rename, `.bak` rotation, recovery, list/rename/delete), `registry.rs` (`SaveSection` {capture, prepare, apply, migrate, check}, stages World/Settled, `plan_restore`, `preflight`, `carry_over_foreign`), `sections.rs` (session, time, weather, session_vehicles v2, active_vehicle, camera, counters, simulation), `ops.rs` (save_session), `restore.rs` (begin_restore, PendingRestore, drive_restore), `quick.rs` (F5/F9, autosave `gameplay.autosave_minutes`), `ui/` (overlay), `fingerprint.rs`.
- Restore: `prepare` fns write NewGameSetup (map, entry, time, weather, `no_vehicle=true`), state InGame (identity transition from a running drive), after HUD up the World stage queues the saved vehicles (spawner), `session_vehicles::state::apply_pending` puts live state on rebuilt vehicles after scripts init, Settled stage (active vehicle, camera, counters, paused).
- Sky and instruments now follow `InGameSessionInfo.sim_elapsed_seconds` (was app virtual time).
- Protocol 1.3.0: `session.save`, `session.load`, `session.saves`, `session.delete_save`, `session.rename_save` (`control_save.rs`).
- Viewpoint store is not on master: no `viewpoint` section yet.

## Status
Done: code, tests (unit, `tests/save_tests.rs`, protocol, pause-overlay UI), docs, live verification (see `docs/src/architecture/session-save.md` section 4 for the measured table), gates run. Live scratch (`~/.local/share/opts/benchmark/session-save/live`) deleted.
