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
| `scene.load_map` | `{path, entry?}` | starts an in-game session on the map (async; poll `state`) | P1 |
| `scene.unload` | — | returns to the main menu | P1 |
| `camera.set_mode` | `{mode, sub?}` | `mode` ∈ `F1…`/`external,ego,cockpit,pax,free` | P1 |
| `camera.teleport` | `{x,y,z,yaw?,pitch?}` | OMSI world coordinates, degrees | P1 |
| `vehicle.set` | `{path}` | replaces the player vehicle with a `.bus` file | P1 |
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

*(Completed in Part E: worked example, error handling, batching guidance.)*

## 10. Path to bevy_remote

Bevy 0.14 has no remote-protocol crate. From 0.15 on, `bevy_remote` offers a JSON-RPC 2.0 reflection API
(`world.query`, `world.get_components`, …). If the engine is upgraded, `entity.*` and `resource.*` should be
re-implemented as thin forwards to it instead of `ControlInspectable`; the method names and results defined
here stay the public contract.
