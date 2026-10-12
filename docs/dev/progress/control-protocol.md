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
- [ ] compile + unit tests green for all of the above
- [ ] headless tests per command group (see tests/ listed below)
- [ ] loading-screen screenshot (black?) investigated live
- [ ] live e2e session (Berlin-Spandau, bus): Home -> plan -> start -> drive 100 m -> scroll settings -> change setting -> pause -> leave
- [ ] docs: architecture/control-protocol.md updated, reference generated, link check, roadmap copy check
- [ ] gates: fmt, clippy, test, doc, check_links, gen_roadmap_copy

## Live findings

(none yet)
