# Control protocol expansion (agent/control-protocol)

Goal: an AI agent can operate the whole app and play the simulation through `opts-cli` without a human.
Branch `agent/control-protocol`, worktree `opts-wt-control-protocol`. Resume from the checklist below.

## Design decisions

- **One descriptor table**: `crates/opts-cli/src/commands.rs` (`COMMANDS`) is the registry of descriptors
  (name, group, summary, arg schema from the `name:type[values]@pos?` notation, since, result, doc, debug).
  It feeds `help`, `commands`, `capabilities`, the CLI parser and the generated reference
  `docs/src/reference/control-protocol.md` (test `reference_page_is_up_to_date`,
  `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`). A game test checks that every registered
  handler has a descriptor and vice versa.
- **Errors**: every wire error has `data.name` and `data.retryable` (`RpcError::structured`); new code
  `1009 GATED` = the input layer refused (modal open, free roam), same gate a key press meets.
- **Protocol version 1.1.0**; new methods carry `since: 1.1`.
- **Gating**: `drive.*` values go into `opts_bevy::vehicle_remote::RemoteControls`, merged inside
  `vehicle_input_system` (same run conditions as keys). Handlers also test the gates and answer 1009.
  Debug commands (skip the gates, marked in the table): `scene.load_map`, `scene.unload`, `vehicle.set`,
  `vehicle.teleport`, `vehicle.reset`, `camera.teleport`, `hud.alert`, `hud.toast`.
- UI additions work on the same messages as clicks (`Activated`, `FocusEvent`, key events, cursor moves).

## Checklist

- [x] descriptor table extended, `commands`/`help` schemas, structured errors, 1009, reference generator
- [x] `RemoteControls` + `drive.*`, `vehicle.teleport/reset`, `camera.look/move`, `snapshot`
- [x] `ui.scrollables/scroll/text/select/toggle/hover/key/drag`, `ui.screen`, `ui.wait_for`, `ui.click count`
- [x] compile + unit tests green for all of the above
- [x] headless tests per command group: `opts-bevy/tests/control_ui_ext.rs` (scroll, list, toggle, select, text, key, hover, gating), `opts-game/tests/control_protocol.rs` (table vs registry, help/commands, drive, gates, triggers, odometer, teleport, reset, wait_for, snapshot, screen), `opts-gui` master-list scroll API, `opts-cli` reference and protocol tests
- [x] loading-screen screenshot investigated live: NOT black in this build (see findings)
- [x] live e2e session verified step by step with opts-cli (Berlin-Spandau, MAN SD77); script `crates/opts-cli/examples/play_session.sh` (whole script run live, exit 0: Home -> drive 100.2 m -> settings -> leave)
- [x] docs: architecture/control-protocol.md updated, reference generated, link check, roadmap copy check
- [x] gates: fmt, clippy, doc, check_links, gen_roadmap_copy clean; `cargo test --workspace --all-features --no-fail-fast`: everything passes except the two pre-existing golden tests `golden_screens` (pause goldens still show *Settings: not available yet*, which the pause overlay no longer shows; unchanged by this branch)

## Live findings

- Real game, throwaway XDG under `~/.local/share/opts/benchmark/control-protocol/`, `--omsi-path nvme2n1 OMSI 2`:
  first-run dialog (`ui.screen` lists its buttons) -> wizard (`ui.select` on `wizard.where.list` / `wizard.what.list`, `wizard.footer.next`, `wizard.footer.drive`) -> loading (`ui.screen.loading`) -> HUD (`ui.wait_for hud_ready`) -> `drive.set gear drive`, `parking_brake false`, `throttle 0.7`, `ui.wait_for distance_above 100` (100.2 m at 47 km/h after 12.6 s) -> brake to 0.5 km/h -> `ui.key Escape` (pause overlay, `sim.get paused true`) -> `pause.settings` -> `ui.scroll to bottom` (132 px) and `item` -> `ui.click texture_filtering/option-8` (`settings.get` = 8), `ui.toggle vsync` -> `settings-overlay.close` -> `pause.leave` -> `dialog.pause.leave.home` -> MainMenu.
  Also live: `drive.trigger` door_front / indicator_left / lights / horn (all `handled: true`), `camera.set_mode free`, `camera.move`, `camera.look` (yaw/pitch and dx/dy in cockpit), `vehicle.teleport`, `vehicle.reset`, `drive.set` refused with 1009 in free roam, with the pause overlay open and with the vehicle menu open (`ui.key AltLeft`), `menu.do close`.
- **Loading-screen screenshot**: three captures during `BuildTerrain`/`SpawnVehicle` show the "Preparing your drive" screen with progress bar, not black. The earlier black result was fixed upstream by the MenuCamera change (`deactivate_menu_camera` only once a scene camera is active); I could not reproduce black. `ui.screen.loading` and `snapshot` also work while loading.
- **App race found (not caused by the protocol)**: when the first-run dialog is answered with *Pick a map and a vehicle* before the first catalogue scan has finished (~8 s on a fresh config), the wizard can stay on *No maps found* (0 entries) although `home` later says 43 maps; leaving and re-entering Drive fills it. Seen 3 of 5 fresh runs. In two of those runs a `ui.navigate home` from the stale wizard panicked the app (`Entity despawned` in a deferred command, `bevy_ecs` command error handler); not reproduced with an idle wait.
- `ui.navigate` used to work while a dialog was open (first-run dialog); it now answers 1009 like a blocked rail click.
