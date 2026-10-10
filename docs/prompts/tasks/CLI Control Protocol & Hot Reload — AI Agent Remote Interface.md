# 🎛️ CLI Control Protocol & Hot Reload

You are implementing two connected features that turn OPTS into a
**remote-controllable, live-editable application**:

1. **A CLI control protocol** that lets any external process — human
   or AI agent — drive the running game entirely from the command
   line: inject input, query state, manipulate the scene, capture
   screenshots, and submit reports.
2. **Hot reloading** for assets, configs, and scene files, so that
   changes to OMSI files (`.cfg`, `.bus`, `.sco`, `.sli`, `.map`,
   `.o3d`, textures) take effect in the running game without a
   restart.

Together, these turn OPTS into an application an AI agent can
**observe, control, and iterate on** in a live loop.

This is a MULTI-PHASE task with checkpoints. The protocol design
is load-bearing — get it wrong and every downstream phase pays for
it.

---

## ⚠️ 0. Design Decisions to Resolve First

Before writing any code, the user must confirm the following.
Report the options and tradeoffs, then STOP.

### 0.1 Transport

How does the CLI talk to the running game?

| Option | Pros | Cons |
| :--- | :--- | :--- |
| **TCP on localhost** (recommended) | Cross-platform, no admin, standard tooling | Requires port management; firewall prompts on Windows |
| **Unix domain socket** | No port, OS-managed permissions | Windows support is awkward (named pipes) |
| **Named pipe (Windows)** / **UDS (Unix)** | Native, fast, no port | Two implementations |
| **stdin/stdout sidecar** | Simplest to implement | Only one client; breaks if game has other I/O |
| **HTTP server** | Familiar tooling (`curl`, `httpie`) | Heavier, overkill for a control protocol |
| **WebSocket** | Streaming, good for events | Requires HTTP upgrade dance |

**Recommendation:** TCP on `127.0.0.1` with a fixed default port
(e.g., `44201`), configurable via CLI flag. Bind to localhost only.
Add an optional auth token for non-loopback cases (which should be
disabled by default).

### 0.2 Protocol Format

| Option | Pros | Cons |
| :--- | :--- | :--- |
| **JSON-RPC 2.0** (recommended) | Standard, self-describing, mature clients | Verbose |
| **Custom line-based** | Tiny, easy to parse | Reinvents the wheel |
| **MessagePack** | Compact, binary-safe | Opaque to humans |
| **gRPC** | Strong typing, codegen | Heavy dependency, proto files |

**Recommendation:** JSON-RPC 2.0. It's the standard for "control a
running thing from outside." AI agents already understand it.
There's a `jsonrpc` crate if needed, but manual implementation is
also tractable.

### 0.3 Response Model

Should commands be request/response, streaming, or both?

**Recommendation:** Both.

- **Request/response** for commands (`query`, `input`, `screenshot`).
- **Server-sent events (SSE)** or a subscription mechanism for
  streaming (log tail, entity changes, FPS samples).

Start with request/response only. Add streaming in a follow-up
phase.

### 0.4 Authentication

**Recommendation:** None by default (loopback only), optional
bearer token via `--cli-token` for cases where the socket is exposed
to a network. Refuse to bind to `0.0.0.0` without a token.

### 0.5 Hot Reload Scope

Which files should hot-reload trigger re-parsing of?

| Category | Reload behavior |
| :--- | :--- |
| Textures (`.dds`, `.tga`, `.bmp`) | Bevy's asset server (built-in) |
| Meshes (`.o3d`, `.x`) | Re-parse + rebuild `Mesh` |
| Configs (`.cfg`, `.bus`, `.sco`, `.sli`) | Re-parse + re-spawn affected entities |
| Map tiles (`.map`) | Re-parse + re-spawn tile contents |
| Rust source (`.rs`) | **Out of scope** — requires rebuild |

**Recommendation:** support all of the above except `.rs`. Rust
source hot-reload requires `dylib`-based hot reload (e.g.,
`dioxus-hot-reload`, `subsecond`, or `hot-lib-reloader`), which is
a separate, much larger project.

### 0.6 Scripting Layer

The prompt title says "hot reloading of scripts." Is this:

- **(A)** Hot reload of OMSI asset files and configs only.
- **(B)** Introduction of an embedded scripting language (Rhai,
  Lua, etc.) for gameplay logic, hot-reloadable.
- **(C)** Both.

**Recommendation:** **A** for this task. **B** is a much larger
design decision that deserves its own ADR and its own milestone.
Document the path to **B** in a follow-up section of the design doc.

### 0.7 Checkpoint

Report your recommendations for each decision with reasoning.
**Wait for user approval before Part A.**

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output targets

- `crates/opts-cli/` — the CLI client binary
- `crates/opts-bevy/src/control/` — the server side
- `crates/opts-bevy/src/hot_reload/` — the reload system
- `crates/opts-game/src/` — wiring
- `docs/src/architecture/control-protocol.md` — the protocol spec
- `docs/src/architecture/hot-reload.md` — the reload design
- `docs/src/adr/0006-control-protocol.md` — ADR for the transport
  and protocol choice
- `docs/src/adr/0007-hot-reload.md` — ADR for the reload scope
- `docs/crates/opts-cli.md` — CLI user manual

### Related context (read first)

- `docs/src/architecture/application-startup.md`
- `docs/src/architecture/camera-system.md`
- `docs/src/formats/text/report-format.md` (if it exists)
- `docs/src/contributing/standards.md`
- `docs/src/adr/0004-logging-architecture.md`
- `crates/opts-bevy/src/` (for the plugin pattern)
- `crates/opts-game/src/` (for wiring)
- `dev/prompts/` (for the library conventions)

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not bind to non-loopback addresses without a token.
- Do not commit any OMSI asset to the repository.
- Do not implement `.rs` source hot-reload. It is out of scope.
- Do not make the CLI a security hole — the server must not accept
  commands that can write to arbitrary paths on the filesystem
  without explicit user permission.
- Do not use `println!` for logging. Use `tracing`.
- Do not block the Bevy render loop on socket I/O. The server must
  be fully asynchronous or run on a dedicated thread.
- Do not implement every command in one pass. Prioritize.
- Do not skip the design phase.

### ✅ Permitted

- Read every file in the repository.
- Add new crates to the workspace.
- Add `tokio`, `serde`, `serde_json`, `jsonrpc-*`, `notify`,
  `tracing-subscriber` (for the log query command), and other
  supporting crates via the workspace dependency table.
- Extend `opts-bevy` with a `ControlPlugin` and a `HotReloadPlugin`.
- Extend `opts-game` to wire them.
- Add a `opts-cli` binary crate.
- Add integration tests that exercise the protocol.
- Add example scripts showing how an AI agent drives the protocol.

---

## 3. Preconditions

Verify and report:

1. `cargo check --workspace` passes.
2. `opts-bevy` has a working plugin entry point.
3. `opts-game` builds and runs.
4. `docs/src/adr/` exists and follows the ADR template.
5. ADR-0004 (logging) exists and describes how logs are captured.
6. The application can capture a screenshot (from F1 reporting or
   similar).
7. `notify` or an equivalent file-watching crate is available in
   the workspace.

If any precondition fails, STOP and report.

---

## 4. Command Surface

The CLI must expose the following commands. Group them by
capability.

### 4.1 Meta

| Command | Params | Returns |
| :--- | :--- | :--- |
| `ping` | — | `{ pong: true, uptime_secs: N }` |
| `version` | — | `{ version, git_hash, build_date }` |
| `help` | `[command]` | description, params, return shape |
| `capabilities` | — | list of command names and groups |
| `shutdown` | `[force]` | triggers app exit |

### 4.2 Query

| Command | Params | Returns |
| :--- | :--- | :--- |
| `state` | — | `{ app_state, camera_mode, fps, frame: N }` |
| `scene` | — | `{ map, tiles: [...], entity_count, asset_count }` |
| `entity.list` | `[filter]` | list of entity IDs with summary |
| `entity.get` | `id` | full component dump for one entity |
| `resource.list` | — | list of resource type names |
| `resource.get` | `name` | serialized resource value |
| `camera.get` | — | `{ mode, sub_index, transform, fov }` |
| `sim.get` | — | `{ paused, sim_time, time_scale, tick }` |
| `log.tail` | `[n]` | last N log lines |
| `log.filter` | `{ target, level, n }` | filtered log lines |
| `asset.list` | `[kind]` | loaded asset paths by kind |
| `asset.get` | `path` | metadata about one asset |

### 4.3 Input Injection

| Command | Params | Returns |
| :--- | :--- | :--- |
| `input.key` | `{ key, duration_ms? }` | ok |
| `input.key_hold` | `{ key }` | ok |
| `input.key_release` | `{ key }` | ok |
| `input.text` | `{ text }` | ok |
| `input.mouse.move` | `{ dx, dy }` | ok |
| `input.mouse.click` | `{ button, pos? }` | ok |
| `input.mouse.scroll` | `{ delta }` | ok |

Keyboard `key` uses a canonical name (`W`, `A`, `F1`, `Escape`)
mapped to Bevy's `KeyCode`.

### 4.4 Simulation Control

| Command | Params | Returns |
| :--- | :--- | :--- |
| `sim.pause` | — | ok |
| `sim.resume` | — | ok |
| `sim.step` | `{ frames }` | ok (advances N frames while paused) |
| `sim.time_scale` | `{ factor }` | ok |
| `sim.set_time` | `{ seconds }` | ok |

### 4.5 Scene Manipulation

| Command | Params | Returns |
| :--- | :--- | :--- |
| `scene.load_map` | `{ path }` | ok (async) |
| `scene.unload` | — | ok |
| `camera.set_mode` | `{ mode, sub? }` | ok |
| `camera.teleport` | `{ x, y, z, yaw?, pitch? }` | ok |
| `vehicle.set` | `{ path }` | ok |
| `entity.spawn` | `{ asset, transform }` | entity_id |
| `entity.despawn` | `{ id }` | ok |
| `entity.set_component` | `{ id, type, value }` | ok |

### 4.6 Reload & Debug

| Command | Params | Returns |
| :--- | :--- | :--- |
| `reload.asset` | `{ path }` | ok |
| `reload.scene` | — | ok (reloads all scene assets) |
| `reload.config` | `{ path }` | ok |
| `watch.add` | `{ path }` | ok |
| `watch.remove` | `{ path }` | ok |
| `watch.list` | — | list of watched paths |
| `debug.wireframe` | `{ enabled }` | ok |
| `debug.normals` | `{ enabled }` | ok |
| `debug.stats` | — | render stats |

### 4.7 Output

| Command | Params | Returns |
| :--- | :--- | :--- |
| `screenshot` | `{ path? }` | saved path |
| `report` | `{ text }` | saved path |

### 4.8 Batch

| Command | Params | Returns |
| :--- | :--- | :--- |
| `batch` | `{ commands: [...] }` | array of results |

Batch lets an AI agent send 20 commands in one round trip. Essential
for a live debugging loop.

---

## 5. Part A — Design

Produce the three design documents.

### Stage A1 — `docs/src/architecture/control-protocol.md`

Contents:

1. Transport (per Section 0.1 decision).
2. Protocol format (JSON-RPC 2.0).
3. Full command surface (Section 4).
4. Request/response examples in JSON.
5. Error codes and their meanings.
6. Rate limiting policy.
7. Security model (loopback only, optional token).
8. Versioning — how the protocol evolves without breaking clients.
9. AI-agent ergonomics — why the protocol is well-suited to
   automated use.

### Stage A2 — `docs/src/architecture/hot-reload.md`

Contents:

1. Scope (per Section 0.5 decision).
2. Watched paths by default (map folder, vehicle folder, asset
   folder).
3. Debounce policy — coalescing file events within a window to
   avoid reload storms.
4. Reload propagation:
   - Which system re-parses which file type.
   - How affected entities are torn down and rebuilt.
   - How the catalogue (`opts-discovery`) is updated.
5. Notification of reload events (log, CLI event stream).
6. Failure handling — if a reloaded file is invalid, keep the old
   asset and log a `WARN`.
7. Interaction with the running simulation — reload while driving.

### Stage A3 — ADRs

Draft:

- `docs/src/adr/0006-control-protocol.md`
- `docs/src/adr/0007-hot-reload.md`

Follow the ADR template. Include alternatives considered and
consequences.

### Stage A4 — Checkpoint

Report the three documents.

**Wait for user approval before Part B.**

---

## 6. Part B — CLI Infrastructure

### Stage B1 — Crate Setup

Create `crates/opts-cli/`:

```
crates/opts-cli/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library for embedding
│   ├── transport.rs         # TCP client
│   ├── protocol.rs          # JSON-RPC types
│   ├── commands.rs          # Command definitions and parsing
│   ├── client.rs            # High-level client API
│   └── output/
│       ├── mod.rs
│       ├── human.rs         # Human-readable output
│       └── json.rs          # JSON output mode
└── tests/
    └── protocol_test.rs
```

Dependencies: `clap`, `serde`, `serde_json`, `tokio` (or
`smol`/`async-std` — pick one), `anyhow`, `tracing`,
`tracing-subscriber`, `opts-logging` if reusable.

### Stage B2 — Transport

Implement the TCP client:

- Connect to `127.0.0.1:44201` (configurable).
- Send JSON-RPC requests.
- Receive responses.
- Handle connection errors gracefully.
- Support one-shot mode (`opts-cli state`) and interactive mode
  (`opts-cli repl`).

### Stage B3 — CLI Surface

Design the CLI:

```
# One-shot
opts-cli state
opts-cli input.key W
opts-cli camera.set_mode F6
opts-cli screenshot --path /tmp/shot.png

# Batch (stdin as JSON)
echo '[{"method":"sim.pause"},{"method":"state"}]' | opts-cli batch

# Interactive REPL
opts-cli repl

# Watch mode — subscribe to events
opts-cli watch --log --scene --sim
```

Output modes:

- `--json` for machine consumption.
- Default: human-readable.
- `--quiet` for scripts.

Exit codes:

- `0` — success
- `1` — command error
- `2` — connection error
- `3` — protocol error

### Stage B4 — REPL

Interactive mode:

- Prompts for commands.
- Tab-completes command names.
- Shows help on `?`.
- Persists history (`~/.local/share/opts/cli_history`).
- Supports multi-line input for `batch`.

### Stage B5 — Test Client

Create an integration test that spins up a mock server, sends
requests, and validates responses. This test runs headlessly in CI.

### Stage B6 — Checkpoint

Report the CLI infrastructure.

**Wait for user approval before Part C.**

---

## 7. Part C — Server Side

### Stage C1 — Control Plugin

Create `crates/opts-bevy/src/control/`:

```
control/
├── mod.rs                 # ControlPlugin
├── server.rs              # TCP listener and connection handling
├── protocol.rs            # Request/response types
├── handlers/
│   ├── mod.rs
│   ├── meta.rs            # ping, version, help
│   ├── query.rs           # state, scene, entity.*, resource.*
│   ├── input.rs           # input.* (writes to Bevy input events)
│   ├── sim.rs             # sim.* (pause, step, time)
│   ├── scene.rs           # scene.*, camera.*, entity.*
│   ├── reload.rs          # reload.*, watch.*
│   ├── output.rs          # screenshot, report
│   └── batch.rs           # batch dispatcher
├── state.rs               # Server-side shared state
└── channel.rs             # Bridge between async server and Bevy
```

The server runs on a dedicated thread with an async runtime
(`tokio`), and sends commands to the Bevy world via a channel.
Responses come back via another channel.

**Critical design point:** the server never touches the Bevy
`World` directly. It sends `ControlCommand` values to a `Receiver`
resource. A Bevy system drains the queue every frame, executes the
commands, and sends responses back to the server thread. This
keeps the async runtime and the ECS separate.

### Stage C2 — Command Dispatch

Implement a Bevy system `handle_control_commands` that:

1. Drains the command channel.
2. Dispatches each command to its handler.
3. Writes the response to the response channel.

Command handlers receive `&mut World` access. Each handler is a
small function. Keep handlers under 80 lines.

### Stage C3 — Input Injection

For `input.key` and friends:

- Synthesize Bevy `KeyboardInput` events.
- Insert them into `Events<KeyboardInput>` before the input system
  runs.
- Alternatively, write to a `ControlInputState` resource that the
  input system consults. This is cleaner but requires the input
  system to be aware of the resource.

**Recommendation:** synthesize Bevy events directly. It's the least
invasive.

For mouse events: same pattern, `Events<MouseButtonInput>`,
`Events<MouseMotion>`, `Events<MouseWheel>`.

### Stage C4 — Query Handlers

For `entity.list`, `entity.get`, `resource.list`, `resource.get`:

Use Bevy's reflection system. Enable `Reflect` on all types that
should be introspectable. Use `bevy_reflect`'s `TypeRegistry` to
enumerate and serialize.

This is more work up front (adding `#[derive(Reflect)]` everywhere)
but gives free introspection and is what makes the CLI useful for
an AI agent.

If full reflection is too heavy for v1:

- `entity.list` returns entity IDs and a `Name`-style label.
- `entity.get` returns only the components explicitly exposed via a
  `ControlInspectable` trait.
- Document the limitation and the path to full reflection.

### Stage C5 — Scene Control

For `scene.load_map`, `vehicle.set`, `entity.spawn`:

Route these to existing subsystems. Do not duplicate logic. A
control command is just a channel to invoke existing behavior.

For `camera.set_mode`, `camera.teleport`: write to `CameraState`.
Same pattern as F-key input.

### Stage C6 — Screenshot Handler

Reuse the screenshot capture flow from F1 reporting. Save to a
given path or to the reports directory. Return the path in the
response.

### Stage C7 — Report Handler

Reuse the F1 report save logic. Return the two paths (`.png` and
`.report`).

### Stage C8 — Lifecycle

The server binds at startup. Log the listening address at `INFO`
level so an AI agent can discover it from the log.

If `--cli-port 0` is passed, bind to an ephemeral port and log the
chosen port. This is useful for tests.

On application exit, close the listener gracefully.

### Stage C9 — Checkpoint

Report the server implementation.

**Wait for user approval before Part D.**

---

## 8. Part D — Hot Reload

### Stage D1 — File Watching

Create `crates/opts-bevy/src/hot_reload/`:

```
hot_reload/
├── mod.rs                 # HotReloadPlugin
├── watcher.rs             # notify-based file watching
├── events.rs              # ReloadEvent types
├── pipeline.rs            # Dispatch by file type
└── dispatch/
    ├── mesh.rs            # .o3d, .x
    ├── texture.rs         # .dds, .tga, .bmp
    ├── config.rs          # .cfg, .bus, .sco, .sli
    ├── tile.rs            # .map
    └── audio.rs           # .wav
```

Use the `notify` crate for cross-platform file watching. Debounce
events with a configurable window (recommended: 250 ms) to coalesce
editor save bursts.

### Stage D2 — Reload Pipeline

For each reloaded file:

1. Emit a `ReloadEvent { path, kind }`.
2. A dispatch system matches the kind and routes to the correct
   pipeline.
3. Pipelines re-parse and update the affected resources or
   entities.

For meshes:

- Re-run the parser.
- Update the Bevy `Asset` (via `Assets<Mesh>::insert`).
- Existing entities referencing the asset update automatically.

For configs:

- Re-parse.
- Mark entities that use the config as dirty.
- A follow-up system rebuilds them.

For map tiles:

- Reload the tile.
- Despawn tile entities.
- Respawn from the new tile.

### Stage D3 — Watch Roots

By default, watch:

- The current map folder (`--map-path`).
- The current vehicle folder (if a vehicle is loaded).
- The `assets/` folder for GUI assets.

Only watch while the app is running. Add CLI flags to override.

### Stage D4 — Reload via CLI

The `reload.*` commands trigger the same pipeline as file events.
This lets an AI agent force a reload without touching the file
system — it can rewrite the file via its own means, then request
the reload explicitly.

### Stage D5 — Failure Handling

If a reloaded file fails to parse:

- Log a `WARN` with the file path and the error.
- Keep the previous asset/state.
- Emit a `ReloadFailed` event with the error.

This is critical: an AI agent iterating on a file will often save
broken intermediate states. The running app must survive them.

### Stage D6 — Checkpoint

Report hot reload.

**Wait for user approval before Part E.**

---

## 9. Part E — AI Agent Integration

### Stage E1 — Protocol Documentation for AI Agents

In `docs/src/architecture/control-protocol.md`, add a section:
"Using the Protocol from an AI Agent." It should include:

- A one-paragraph overview: what an AI agent can do with this.
- The connect-dance: `opts-cli` versus raw TCP, and when to use
  each.
- A worked example: "the agent wants to load a map, drive the
  vehicle to a specific location, take a screenshot, submit a
  report." This shows a sequence of 8–12 commands.
- Error handling: what to do when a command fails.
- Rate limits and how to batch commands efficiently.

### Stage E2 — Example Client Scripts

Create `crates/opts-cli/examples/`:

- `drive_and_screenshot.rs` — load a map, drive for N seconds,
  screenshot.
- `reload_iterate.rs` — watch a config file, detect changes, verify
  the app reloaded.
- `scene_probe.rs` — enumerate scene entities, print a summary.

These are not production tools. They are documentation-as-code for
AI agents to reference.

### Stage E3 — README

Create `crates/opts-cli/README.md` with:

- What the CLI is.
- How to start the app with the control server enabled.
- A quick tour of the most-used commands.
- A pointer to the full protocol doc.

### Stage E4 — `docs/crates/opts-cli.md`

The crate doc for `opts-cli`, following OPTS conventions.

### Stage E5 — Checkpoint

Report the docs and examples.

**Wait for user approval before Part F.**

---

## 10. Part F — Verification

### Stage F1 — Manual Protocol Test

Start the app:

```
cargo run -p opts-game -- --map-path /path/to/Waldhofen --cli-port 44201
```

Then, in another terminal:

```
opts-cli ping
opts-cli state
opts-cli camera.set_mode F6
opts-cli input.key W --duration 2000
opts-cli sim.pause
opts-cli sim.step 60
opts-cli screenshot --path /tmp/probe.png
opts-cli report --text "Test report from CLI"
```

Verify each returns success and produces the expected effect.

### Stage F2 — Hot Reload Test

1. Start the app with a map loaded.
2. Open a config file (e.g., `model_GN92_main.cfg`) in an editor.
3. Change a mesh reference or material.
4. Save.
5. Confirm the app logs a reload event and the scene updates.

Repeat for:

- A `.o3d` mesh (tint a material, save, observe).
- A `.sli` spline (change profile, save, observe).
- A `.map` tile (move a scenery object, save, observe).

### Stage F3 — Broken File Test

1. Truncate a `.cfg` file.
2. Save.
3. Confirm the app logs `WARN` and keeps the previous state.

### Stage F4 — AI Agent Simulation

Write a shell script that drives the protocol as an AI agent would:

```
#!/bin/bash
opts-cli scene.load_map "/path/to/map" || exit 1
sleep 2
opts-cli vehicle.set "/path/to/vehicle" || exit 1
opts-cli camera.set_mode F3
opts-cli sim.resume
opts-cli input.key W --duration 5000
opts-cli sim.pause
opts-cli screenshot --path /tmp/ai_probe_$(date +%s).png
opts-cli entity.list --filter "category=vehicle" | jq '.result'
```

Run it end to end. Confirm the whole flow works headless-ish.

### Stage F5 — Automated Tests

Integration tests in `crates/opts-cli/tests/`:

- Start a stub server, send each command, verify the response.
- Test batching.
- Test malformed requests return proper error codes.
- Test reconnection after server restart.

Integration tests in `crates/opts-bevy/tests/control.rs`:

- Build a headless Bevy app with `ControlPlugin`.
- Send commands via the channel.
- Verify handlers do the right thing.

### Stage F6 — Final Report

Produce `docs/src/meta/control-protocol-YYYY-MM-DD.md`:

- Which commands were implemented.
- Which were deferred and why.
- Which hot reload paths work.
- Test results.
- Sample command sequences that work end to end.
- Any limitations.

---

## 11. Reporting Cadence

1. Section 0 decisions — approval before Part A
2. Part A (design docs + ADRs) — approval before Part B
3. Part B (CLI infrastructure) — approval before Part C
4. Part C (server) — approval before Part D
5. Part D (hot reload) — approval before Part E
6. Part E (docs and examples) — approval before Part F
7. Part F (verification) — final report

Format:

```
## Control Protocol — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Commands implemented:** <count>
**Reload paths supported:** <list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 12. Exit Criteria

- ✅ Transport, protocol format, and auth decisions resolved in
      ADR-0006.
- ✅ Hot reload scope resolved in ADR-0007.
- ✅ `opts-cli` crate exists, builds, and connects to the running
      app.
- ✅ All commands from Section 4 implemented (or documented as
      deferred with reason).
- ✅ Server runs on a dedicated thread; Bevy render loop never
      blocks on I/O.
- ✅ Input injection produces the same effect as real keyboard and
      mouse input.
- ✅ Hot reload works for `.o3d`, `.cfg`, `.sli`, `.map`, textures.
- ✅ Broken files log warnings and preserve previous state.
- ✅ `docs/src/architecture/control-protocol.md` exists.
- ✅ `docs/src/architecture/hot-reload.md` exists.
- ✅ `docs/src/adr/0006-*.md` and `docs/src/adr/0007-*.md` exist.
- ✅ `crates/opts-cli/README.md` and `docs/crates/opts-cli.md`
      exist.
- ✅ Example scripts under `crates/opts-cli/examples/` run
      successfully.
- ✅ Integration tests in `crates/opts-cli/tests/` and
      `crates/opts-bevy/tests/control.rs`.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No `println!` in library code.
- ✅ No `unsafe`.
- ✅ Loopback binding only; no external network exposure without a
      token.

When finished, respond with the summary from Stage F6.

Then respond with the exact string:
"CONTROL PROTOCOL COMPLETE — ready for user review"