# 🎛️ Control Protocol

**Status:** 🟡 Draft (design for M-0043) · **Decision record:** [ADR-0006](../adr/0006-control-protocol.md) · **Client:** `opts-cli` · **Server:** `opts-bevy::control`

A running OPTS instance can be driven from outside: a human at a terminal or an AI agent in a loop can inject
input, query state, move the camera, pause and step the simulation, capture screenshots, file reports and
trigger reloads. The server is **opt-in** and **loopback-only by default**.

## 1. Transport

| Property | Value |
| :--- | :--- |
| Transport | TCP |
| Default bind | `127.0.0.1:44201` (only when `--cli-port <N>` is given) |
| Ephemeral port | `--cli-port 0`: the OS picks a port; the address is logged at `INFO` as `control server listening addr=127.0.0.1:NNNNN` |
| Non-loopback | refused unless `--cli-token <TOKEN>` is set (`--cli-bind 0.0.0.0` additionally required) |
| Framing | UTF-8 JSON, one message per line (`\n`-terminated), at most 1 MiB per line |
| Concurrency | one thread per connection; any number of clients; requests of one connection are answered in order |

The server never touches the Bevy `World` from its threads. It parses a request, sends a `ControlCommand`
through a channel and waits for the reply; a Bevy system drains the channel once per frame
(`handle_control_commands`, schedule `First`), runs the handler with `&mut World` access and sends the reply
back. A command that is not answered within **5 s** (for example because the app is frozen) fails with
error `1005`. Socket I/O can therefore never block rendering.

## 2. Message format — JSON-RPC 2.0

```json
{"jsonrpc":"2.0","id":1,"method":"camera.set_mode","params":{"mode":"F6"}}
{"jsonrpc":"2.0","id":1,"result":{"mode":"free"}}
{"jsonrpc":"2.0","id":2,"error":{"code":1004,"message":"entity 99 not found","data":{"id":99}}}
```

- `params` is an object (positional arrays are rejected with `-32602`). Unknown members are ignored so newer
  clients can talk to older servers.
- A request without `id` is a *notification*: executed, never answered.
- A line that is a JSON **array** is a standard JSON-RPC batch; the reply is an array. The `batch` method
  (§4.8) is the sequential variant with `stop_on_error`.

## 3. Errors

| Code | Name | Meaning |
| ---: | :--- | :--- |
| -32700 | Parse error | the line is not valid JSON |
| -32600 | Invalid request | not a JSON-RPC request object |
| -32601 | Method not found | unknown method (see `capabilities`) |
| -32602 | Invalid params | missing, mistyped or out-of-range parameter |
| -32603 | Internal error | handler panicked or failed unexpectedly |
| 1001 | Not ready | the command needs a state the app is not in (for example `vehicle.set` in the menu) |
| 1002 | Forbidden path | path outside the allowed read/write roots (§7) |
| 1003 | Unauthorized | token missing or wrong |
| 1004 | Not found | entity, asset or file does not exist |
| 1005 | Timeout | the app did not process the command within 5 s |
| 1006 | Busy | rate limit exceeded or command queue full |
| 1007 | Unsupported | the command exists in the protocol but this build does not implement it |
| 1008 | Failed | the operation ran and failed (`data` carries the cause) |

## 4. Command surface

Methods are grouped by capability; `capabilities` returns the same table at runtime with a `since` protocol
version per method. **Priority** marks the delivery order: **P1** is implemented in the first pass, **P2** is
specified and answers `1007` until implemented.

### 4.1 Meta

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `ping` | — | `{pong:true, uptime_secs}` | P1 |
| `version` | — | `{version, git_hash, build_date, protocol_version}` | P1 |
| `help` | `[command]` | `{command, summary, params, result}` | P1 |
| `capabilities` | — | `{groups:{name:[{method,since,priority}]}}` | P1 |
| `auth` | `{token}` | `{ok:true}` — required first when `--cli-token` is set | P1 |
| `shutdown` | `[force]` | `{ok:true}` then the app exits (`force` skips the clean shutdown path) | P1 |

### 4.2 Query

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `state` | — | `{app_state, camera_mode, fps, frame, map, vehicle}` | P1 |
| `scene` | — | `{map, tiles:[{x,y,entities}], entity_count, asset_count, stats}` | P1 |
| `camera.get` | — | `{mode, sub_index, transform:{translation,rotation}, fov}` | P1 |
| `sim.get` | — | `{paused, sim_time, time_scale, tick}` | P1 |
| `log.tail` | `[n=100]` | `{lines:[{ts,level,target,message}]}` | P1 |
| `log.filter` | `{target?, level?, contains?, n?}` | as `log.tail` | P1 |
| `entity.list` | `[filter]` | `{entities:[{id,name,summary}]}`; filter keys: `name`, `category` | P2 |
| `entity.get` | `id` | `{id, components:{type:value}}` for components implementing `ControlInspectable` | P2 |
| `resource.list` | — | `{resources:[name]}` | P2 |
| `resource.get` | `name` | serialised value | P2 |
| `asset.list` | `[kind]` | `{assets:[{kind,path}]}` | P2 |
| `asset.get` | `path` | `{kind, size, vertices?, dims?}` | P2 |

### 4.3 Input injection

Injected input is delivered as ordinary Bevy input events (`KeyboardInput`, `MouseButtonInput`,
`MouseMotion`, `MouseWheel`, text via `KeyboardInput.logical_key`), so it passes through exactly the same
systems — including the modal gating — as a physical keyboard.

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `input.key` | `{key, duration_ms?=50}` | press, hold for the duration, release | P1 |
| `input.key_hold` | `{key}` | press without release | P1 |
| `input.key_release` | `{key}` | release | P1 |
| `input.text` | `{text}` | typed into the focused text field | P1 |
| `input.mouse.move` | `{dx, dy}` | relative motion | P1 |
| `input.mouse.click` | `{button, pos?}` | `button` ∈ `left,right,middle` | P1 |
| `input.mouse.scroll` | `{delta}` | wheel lines | P1 |

Key names: `A`–`Z`, `0`–`9`, `F1`–`F12`, `Escape`, `Enter`, `Space`, `Tab`, `Backspace`, `ArrowUp/Down/Left/Right`,
`ShiftLeft`, `ControlLeft`, `AltLeft`; unknown names fail with `-32602` and the list of valid names in `data`.

### 4.4 Simulation control

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `sim.pause` | — | pauses `Time<Virtual>` | P1 |
| `sim.resume` | — | resumes | P1 |
| `sim.step` | `{frames}` | advances exactly N frames while paused; replies after the last one | P1 |
| `sim.time_scale` | `{factor}` | relative speed, 0.0 < factor ≤ 20 | P1 |
| `sim.set_time` | `{seconds}` | sets the session clock (sky, schedule) | P2 |

### 4.5 Scene manipulation

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `scene.load_map` | `{path, entry?}` | leaves the current session if any, starts one on the map folder and answers when the HUD is up (≤ 120 s) | P1 |
| `scene.unload` | — | returns to the main menu | P1 |
| `camera.set_mode` | `{mode, sub?}` | `mode` ∈ `ego, cockpit, external, pax, free` or `1`–`5` (the camera number keys); injects the key, so unavailable modes answer `1001`; answers `{mode, sub_index}` two frames later | P1 |
| `camera.teleport` | `{x,y,z,yaw?,pitch?}` | switches to free roam and moves the camera to the **absolute** OMSI world position (metres; tile `(i,j)` starts at `300·i`, `300·j`); `yaw` is a compass heading in degrees (0 = north, clockwise), `pitch` degrees up | P1 |
| `menu.do` | `{command}` | drives the in-game vehicle menu (Left Alt): `open`, `close`, `spawn`, `vehicle <text>`, `repaint <text>`, `entry <text>`, `config`, `toggle <var>`, `done`, `confirm`, `despawn`, `back`; see [vehicle menu](in-game-vehicle-menu.md) | P2 |
| `menu.state` | — | page, chosen vehicle/repaint/entry and the free entry points of the vehicle menu | P2 |
| `render.look` | `[look]` | `{look, previous, available}`; without `look` it reads the colour look, with `look` ∈ `vivid, natural, bevy` it sets it: live on every camera and the sky, and saved in the settings; unknown names answer `-32602` | P1 |
| `vehicle.get` | — | `{x, y, z, heading_deg, speed_kmh}` of the player vehicle (absolute OMSI coordinates) | P1 |
| `vehicle.script_vars` | `{contains?}` | the player vehicle's script variables `{count, variables, ops_executed}` (substring filter) | P1 |
| `vehicle.set` | `{path}` | takes a `.bus` file and restarts the session with it (answers when it is up) | P1 |
| `entity.spawn` | `{asset, transform}` | `{id}` | P2 |
| `entity.despawn` | `{id}` | ok | P2 |
| `entity.set_component` | `{id, type, value}` | ok (only types registered as settable) | P2 |

### 4.6 Reload & debug

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `reload.asset` | `{path}` | forces the hot-reload pipeline for one file | P1 |
| `reload.scene` | — | reloads every resident tile and the vehicle | P1 |
| `reload.config` | `{path}` | alias of `reload.asset` for `.cfg/.bus/.sco/.sli` | P1 |
| `watch.add` / `watch.remove` | `{path}` | adds/removes a watch root | P1 |
| `watch.list` | — | `{roots:[path]}` | P1 |
| `debug.wireframe` | `{enabled}` | toggles wireframe rendering | P2 |
| `debug.normals` | `{enabled}` | toggles normal visualisation | P2 |
| `debug.stats` | — | `{fps, frame_ms, entities, draw_calls?, descramble:{decoded,skipped}}` | P2 |

### 4.7 Output

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `screenshot` | `{path?}` | `{path}`; default: the reports directory | P1 |
| `report` | `{text}` | `{png, report}` — same files as F1 | P1 |

### 4.8 Batch

`batch` takes `{commands:[{method,params}], stop_on_error?=false}` and returns `{results:[{result|error}]}`.
Commands run sequentially in order; at most 100 per batch. Because each command completes in the frame in
which it is executed (except `sim.step` and `scene.load_map`), a batch of 20 usually costs 1–3 frames.

### 4.9 UI by id

The UI can be read and driven without coordinates. Every interactive element has a **stable path id** (`menu.start`, or an automatic `parent/role:slug` such as `button:start-drive`; duplicates get `-2`, `-3`). Screens that must survive copy changes and language switches set explicit ids; the automatic slug comes from the message key when there is one, else from the label text. The actions send exactly the messages a pointer release, a keyboard or a gamepad would, so they pass the same gating, and they answer at once - the effect is visible one or two frames later (read it back with `ui.state`).

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `ui.tree` | `{visible_only?=false}` | `{focus, count, tree:[node]}` where a node is `{id, role, label, value?, state, rect, focusable, children}`; `role` is one of `button, checkbox, radio, switch, textbox, combobox, option, tablist, tab, tabpanel, tree, treeitem, listitem, dialog, toast, region`; `state` is `{hovered, focused, pressed, selected, checked, expanded, disabled, disabled_reason, error, visible}`; `rect` is `[x, y, w, h]` in logical pixels or `null` before layout | P1 |
| `ui.state` | `{id}` | one node without children; `1004` for an unknown id | P1 |
| `ui.click` | `{id}` | activates the element (a text field gets focus); `1001` when it is hidden or disabled (`data.reason` carries the reason), `1004` for an unknown id | P1 |
| `ui.focus` | `{id}` | moves keyboard focus; `1001` when the element cannot take focus now (hidden, disabled without a reason, behind a dialog) | P1 |
| `ui.action` | `{action, arg?}` | sends a `UiAction`: `navigate up\|down\|left\|right`, `next-region`, `prev-region`, `next`, `previous`, `next-tab`, `prev-tab`, `confirm`, `back`, `search`, `menu`, `help`, `scroll page-up\|page-down\|home\|end\|up\|down`; `-32602` with the valid spellings in `data` otherwise | P1 |

From the shell: `opts-cli ui.tree --visible_only true`, `opts-cli ui.click menu.start`, `opts-cli ui.action navigate right`. A scripted keyboard traversal is `ui.action next` repeated while reading `ui.tree`'s `focus`.

## 5. Rate limiting

Per connection: token bucket of 200 requests/s with a burst of 400; beyond that `1006 Busy`. The Bevy side
executes at most 64 commands per frame; the rest wait in the queue (bounded at 1,024; a full queue answers
`1006`). `input.key` with a `duration_ms` schedules the release on a timer, not by blocking the reply.

## 6. Versioning

`version` reports `protocol_version` (semver). Minor versions only **add** methods, optional parameters and
result members; clients ignore unknown result members and servers ignore unknown parameters. A breaking change
bumps the major version and changes the default port. `capabilities` lets a client feature-detect instead of
comparing versions.

## 7. Security model

- Default is **off**. Without `--cli-port` no socket exists.
- Loopback only. A non-loopback bind requires `--cli-token` and `--cli-bind`; the token is compared in
  constant time and required via `auth` before any other call.
- **Filesystem:** the server never accepts a command that writes outside the *reports directory* or
  directories passed with `--cli-allow-dir`. Reads (`scene.load_map`, `vehicle.set`, `watch.add`,
  `reload.asset`) must resolve (after canonicalisation, so `..` and symlinks are resolved) below the OMSI
  installation root or an allowed directory; otherwise `1002`.
- No command executes code or shell commands.
- All requests and errors are logged at `DEBUG`/`WARN` under target `opts_bevy::control`.

## 8. Why this suits AI agents

- Line-delimited JSON is trivially produced and parsed by an agent or `jq`; every reply is machine-readable
  with stable error codes.
- `capabilities` and `help` are self-describing, so an agent can discover the surface without the doc.
- `batch` and `sim.step` give deterministic, low-round-trip loops: *pause → act → step N → observe*.
- `screenshot` and `log.tail` close the observe loop (look at the frame, read the warnings).
- Hot reload (see [Hot Reload](hot-reload.md)) closes the edit loop: write a file, call `reload.asset`, inspect.

## 9. Using the protocol from an AI agent

**What an agent can do.** Start or find a running OPTS, load a map, put the camera anywhere, drive the
bus with injected keys, pause and single-step time, look at the result (screenshot) and read what the game
complained about (`log.filter`), edit a content file and have it reloaded, and file a report — all without
a human at the keyboard.

**Connecting.** Start the game with `opts-game --cli-port 44201` (add `--cli-token <T>` if the port is
reachable by others). The listening address is logged at `INFO` (`control server listening addr=…`), which is
how an agent finds an ephemeral port (`--cli-port 0`).

| Use | When |
| :--- | :--- |
| `opts-cli <method> [args]` | quick one-shot commands from a shell; `--json` for machine output; exit codes 0/1/2/3 tell success, command error, no connection, bad protocol |
| `opts-cli batch` (stdin) / `batch` method | several commands in one round trip |
| Raw TCP | an agent runtime with sockets: send one JSON line, read one JSON line — `printf '{"jsonrpc":"2.0","id":1,"method":"ping"}\n' \| nc 127.0.0.1 44201` |
| `opts_cli::client::Client` | Rust code; see `crates/opts-cli/examples/` |

**A worked session** (drive to a spot, look, report). Each line is one request; results are abbreviated.

```text
 1 ping                                         → {"pong":true,"uptime_secs":3}
 2 capabilities                                 → which methods exist and are implemented
 3 scene.load_map  path="…/maps/BRT Berlin"     → {"map":"BRT Berlin","tiles_loaded":8,…}   (≈ 2–20 s)
 4 camera.set_mode external
 5 sim.resume
 6 input.key_hold W
 7 sim.step 120                                 → only if the sim is paused; otherwise just wait
 8 input.key_release W
 9 sim.pause
10 camera.get                                   → where am I?
11 screenshot                                   → {"path":"…/reports/screenshot_2026-10-10_16-01-40.png"}
12 log.filter level=warn n=10                   → what did the game complain about?
13 report text="the junction at … has no road surface"   → {"png":…,"report":…}
```

As one batch, steps 4–12 cost a single round trip:

```json
{"jsonrpc":"2.0","id":9,"method":"batch","params":{"commands":[
  {"method":"camera.set_mode","params":{"mode":"external"}},
  {"method":"sim.pause"},
  {"method":"sim.step","params":{"frames":60}},
  {"method":"screenshot"},
  {"method":"log.filter","params":{"level":"warn","n":10}}]}}
```

**Deterministic loops.** Prefer *pause → act → `sim.step N` → observe* over sleeping: with the simulation
paused, input injected before a step takes effect in exactly those frames.

**When a command fails** (error `code`): `-32602` — fix the parameter (the message names it; `input.key`
lists valid keys in `data`); `1001` — the app is in the wrong state (load a map first, or the camera mode is
unavailable); `1002` — the path is outside the allowed directories (use `--cli-allow-dir` when starting the
game); `1004` — nothing there; `1005` — the app did not answer: it is frozen or loading, retry after
`ping`; `1006` — slow down or batch; `1007` — not implemented in this build (`capabilities` shows which);
`1008` — the operation ran and failed (reload of a broken file: the previous content is still in the game).
Connection errors (exit code 2) mean the game is not running or the port differs.

**Rate limits.** 200 requests/s per connection with a burst of 400; the game runs at most 64 commands per
frame. Batches of up to 100 commands avoid both limits.

**Editing content.** Write the file with your own tools; the game's watcher reloads it after a 250 ms quiet
period. To be sure, call `reload.asset {path}` — it answers with what was rebuilt, or with error `1008` and
the parser's message when the file is broken. A bad save never breaks the running scene.

## 10. Path to bevy_remote

Bevy 0.14 has no remote-protocol crate. From 0.15 on, `bevy_remote` offers a JSON-RPC 2.0 reflection API
(`world.query`, `world.get_components`, …). If the engine is upgraded, `entity.*` and `resource.*` should be
re-implemented as thin forwards to it instead of `ControlInspectable`; the method names and results defined
here stay the public contract.
