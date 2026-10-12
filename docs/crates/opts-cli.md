# `opts-cli`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L3 · Applications (the protocol module is shared downward) |
| **Role** | Game arguments, control-protocol types, remote-control client |
| **Kind** | Library (`opts_cli`) + binary (`opts-cli`, feature `client`) |
| **Status** | 🟢 Implemented (protocol 1.0) |
| **Last Updated** | 2026-10-10 |

## 🎯 Purpose

`opts-cli` is the command-line surface of OPTS:

1. [`args`](#game-arguments) — `CliArgs`, the argument schema of the game binary (used by `opts-game`).
2. [`protocol`](#protocol-types) — JSON-RPC 2.0 request/response/error types and the error codes of the
   [control protocol](../src/architecture/control-protocol.md). The server in `opts-bevy::control` uses the
   same types, so client and server cannot drift.
3. The **client** (feature `client`, on by default): [`transport`], [`client`], [`commands`], [`output`],
   [`repl`], [`run`] and the binary `opts-cli`.

Game crates depend on `opts-cli` with `default-features = false`, so they do not build the client's
line-editor dependency.

## 🧩 Modules

| Module | Responsibility |
| :--- | :--- |
| `args` | `CliArgs` (clap): map/vehicle/screenshot/log/window flags plus the control and hot-reload flags |
| `protocol` | `Request`, `Response`, `RpcError`, `code::*`, `parse_line`, `PROTOCOL_VERSION`, limits |
| `transport` | blocking TCP, one JSON message per line, 1 MiB line cap |
| `client` | `Client::call`, `Client::batch`; request ids, `auth`, one reconnect after a broken connection |
| `commands` | the table of all 54 methods (incl. the five `ui.*` commands) (`COMMANDS`) and `build_params` (command-line words → JSON) |
| `output` | human and JSON rendering of results and errors |
| `repl` | interactive mode (`rustyline`): completion, history, multi-line JSON |
| `run` | argument handling and exit codes of the binary |

## Game arguments

`CliArgs` flags relevant to remote control and reload:

| Flag | Default | Meaning |
| :--- | :--- | :--- |
| `--cli-port <PORT>` | off | start the control server (`0` = any free port) |
| `--cli-bind <ADDR>` | `127.0.0.1` | bind address; non-loopback needs `--cli-token` |
| `--cli-token <TOKEN>` | none | shared secret checked by `auth` |
| `--cli-allow-dir <DIR>` | none | extra directory for reads/writes (repeatable) |
| `--no-hot-reload` | off | disable the file watcher |
| `--hot-reload-root <DIR>` | none | extra recursive watch root (repeatable) |
| `--hot-reload-debounce-ms <MS>` | 250 | quiet period before a changed file is reloaded |

The other flags (`--map-path`, `--omsi-path`, `--vehicle-path`, `--autostart`, `--camera`, `--screenshot`,
`--screenshot-delay`, `--recent-map`, `--log-level`, `--log-file`, `--fullscreen`, `--windowed`,
`--window-size`, `--headless`) are unchanged; see `opts-game --help`.

## Protocol types

```rust
use opts_cli::protocol::{parse_line, Incoming, Request, code};
let line = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
assert!(matches!(parse_line(line), Ok(Incoming::Single(_))));
assert_eq!(code::UNAUTHORIZED, 1003);
```

## The `opts-cli` binary

```text
opts-cli [--host H] [--port P] [--token T] [--json] [--quiet] [--timeout-ms N] <command> [arguments…]
```

* **Commands** are protocol methods (`opts-cli list` prints them with descriptions); `repl`, `batch` and
  `list` are handled locally. A method this build does not know is still sent, with `--name value` /
  `name=value` pairs as parameters (JSON when it parses), so the client works against newer servers.
* **Arguments**: positional values, `--flag value`, `--flag=value`, `name=value`, or `--json '{…}'` for a
  whole parameter object. Booleans accept `true/false/on/off/1/0`; a bare `--flag` means `true`.
* **Output**: human text by default (`key: value` lines, one log line per row), `--json` prints
  `{"result": …}` or `{"error": {…}}`, `--quiet` prints nothing on success.
* **Exit codes**: `0` success · `1` command error (server error object, bad arguments) · `2` connection
  error · `3` protocol error (reply that is not valid JSON-RPC, id mismatch).
* **Timeouts**: `--timeout-ms` (10 s) applies to connect and read; `sim.step`, `scene.load_map`,
  `scene.unload` and `vehicle.set` automatically allow 120 s.
* **REPL** (`opts-cli repl`): Tab completes method names, `? [command]` shows help, `exit`/Ctrl-D leave,
  unbalanced `{`/`[`/quotes continue on the next line. History: `~/.local/share/opts/cli_history`.

## Library use

```rust,no_run
use opts_cli::client::{Client, ClientConfig};
use serde_json::{json, Value};

let mut c = Client::new(ClientConfig::default());
c.call("scene.load_map", json!({"path": "/path/to/map"}))?;
let state = c.call("state", Value::Null)?;
println!("{}", state["camera_mode"]);
# Ok::<(), opts_cli::client::ClientError>(())
```

## ✅ Testing

* `tests/protocol_test.rs` runs the client against an in-process mock server: calls, error data, refused
  connections, garbage replies, batches, reconnection after a restart, tokens, exit codes.
* Unit tests cover protocol parsing, parameter building, output formatting and REPL helpers.
* The server side is tested in `opts-bevy` (`tests/control.rs`, `tests/hot_reload.rs`) against the real socket.

## 📦 Dependencies

`clap`, `serde`, `serde_json`; with `client`: `rustyline`, `directories`. No async runtime.

## 🔗 Related

[Control Protocol](../src/architecture/control-protocol.md) · [Hot Reload](../src/architecture/hot-reload.md) ·
[ADR-0006](../src/adr/0006-control-protocol.md) · [ADR-0007](../src/adr/0007-hot-reload.md)
