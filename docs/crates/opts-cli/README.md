# `opts-cli`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L0 · Shared Foundations |
| **Role** | Game arguments, control-protocol types, remote-control client (manual: [`../opts-cli.md`](../opts-cli.md)) |
| **Kind** | Library (`opts_cli`) + binary `opts-cli` |
| **Status** | 🟢 Implemented (`CliArgs`, dimensions, flags) |
| **Last Updated** | 2026-10-10 |

---

## 🎯 Purpose

`opts-cli` provides unified command-line argument parsing and flag definitions for the Open Public Transport Simulator. It isolates argument schemas from runtime engine components, allowing both the root binary (`src/main.rs`) and `opts-game` to ingest options consistently.

## 📤 Public API

```rust
use opts_cli::CliArgs;

let args = CliArgs::parse_args();
let (width, height) = args.parse_dimensions();
```

### Supported CLI Flags

| Flag | Type | Default | Description |
| :--- | :---: | :--- | :--- |
| `--map-path <PATH>` | `Option<PathBuf>` | None | Override map directory for background preview |
| `--recent-map <PATH>` | `Option<PathBuf>` | None | Persist path as the last-loaded recent map in user settings |
| `--log-level <LEVEL>` | `String` | `"info"` | Global log filter level (`trace`, `debug`, `info`, `warn`, `error`) |
| `--log-file <PATH>` | `Option<PathBuf>` | Platform default | Directory where subsystem log files are written |
| `--fullscreen` | `bool` | `false` | Launch window in borderless fullscreen mode |
| `--windowed` | `bool` | `false` | Explicitly request windowed mode (overrides fullscreen) |
| `--window-size <WxH>` | `String` | `"1920x1080"` | Initial window dimension (e.g. `1920x1080`); sizes below the minimum window of 1024x640 are raised to it |
| `--headless` | `bool` | `false` | Run without opening an OS display window (ideal for CI testing) |
| `--cli-port`, `--cli-bind`, `--cli-token`, `--cli-allow-dir` | | off | Control server (see the [manual](../opts-cli.md)) |
| `--no-hot-reload`, `--hot-reload-root`, `--hot-reload-debounce-ms` | | on, 250 ms | Hot reload of edited content |

## 📦 Dependencies

| Dependency | Why |
| :--- | :--- |
| `clap` (`derive`) | Robust, declarative argument parsing and auto-generated `--help` documentation |

## 🔗 Cross-References

- [opts-game](../opts-game.md)
- [Application Startup Architecture](../../src/architecture/application-startup.md)
- [Crate Architecture ADR-0005](../../src/adr/0005-crate-architecture.md)
