# ADR-0004: Unified Logging Architecture & Diagnostic Subsystem

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0004 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-08 |
| **Deciders** | OPTS Architecture Team & Core Contributors |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

As OPTS evolves into a multi-crate real-time simulation platform combining pure parsers (`opts-core`), deterministic physics (`opts-sim`), Bevy ECS rendering bridges (`opts-bevy`), Material Design 3 GUI (`opts-gui`), and executables (`opts-game`, `opts-tools`), robust diagnostics and telemetry are critical.

Previously, logging had several systemic issues:
1. **Ad-Hoc and Split Diagnostics:** Crates declared disparate logging configurations; some used `bevy::log`, others imported `tracing-subscriber` directly, while CLI utilities lacked structured output.
2. **Game-Loop Latency Risks:** Synchronous disk writes during logging can induce micro-stutters and frame drops in real-time 60 Hz rendering and physics simulation loops.
3. **Log Interleaving & File Bloat:** High-volume debug logs from 3D loaders or coordinate math (`opts-bevy`) overwhelmed general application logs, making parser verification or physics anomalies hard to trace.
4. **Lack of Post-Mortem Crash Reporting:** Unhandled panics dumped stack traces to stderr, which are lost when launching binaries from desktop environments without an attached terminal.
5. **Crate Boundary Isolation:** Foundational domain crates (`opts-core`, `opts-sim`) must not depend on Bevy, yet need to emit telemetry that the Bevy application can capture seamlessly.

A dedicated foundational crate, `opts-logging`, positioned at Layer 0 (zero OPTS dependencies), is required to provide non-blocking multi-target logging, subsystem-specific log file routing, log rotation and retention, and crash panic hooks.

---

## 🎯 Decision

**We will implement `opts-logging` as a zero-dependency foundational Layer 0 crate utilizing the Rust `tracing` ecosystem (`tracing`, `tracing-subscriber`, `tracing-appender`) with non-blocking writers, subsystem file routing, log rotation, and panic telemetry.**

Specifically:
1. **Zero OPTS Coupling:** `opts-logging` will have zero dependencies on any OPTS internal crate, ensuring it can be consumed universally.
2. **Pre-Engine Initialization:** `opts_logging::init()` initializes the global `tracing` subscriber *before* Bevy is initialized. Bevy seamlessly inherits this global subscriber, avoiding duplicate subscriber errors.
3. **Non-Blocking Asynchronous Appenders:** All file and console writes execute via background worker threads using `tracing_appender::non_blocking`, preventing disk I/O from stalling the simulation or rendering loops.
4. **Lifecycle Guard:** `init()` returns an RAII `LogGuard`. When dropped during application shutdown, it flushes all pending in-flight log records before process exit.
5. **Subsystem File Routing Layer:** Log records are routed based on `Metadata::target()` to dedicated subsystem log files as well as a centralized aggregate file.
6. **Integrated Crash Telemetry:** A panic hook records fatal panic messages and backtraces to `panics.log` and flushes all non-blocking buffers before delegating to the default panic handler.

---

## 🧭 Subsystem Routing Table

Logs are written to the platform data directory (e.g. `~/.local/share/opts/logs/` or `%APPDATA%/opts/logs/`):

| File | Target Filter | Purpose |
| :--- | :--- | :--- |
| `opts.log` | `*` (All targets at configured global level) | Unified aggregate session log |
| `opts-core.log` | `opts_core::*` | Format parsing, chunk decoding, asset metadata |
| `opts-sim.log` | `opts_sim::*` | Kinematics, drivetrain torque, Ackermann steering, physics |
| `opts-bevy.log` | `opts_bevy::*`, `wgpu`, `naga`, `bevy_render` | Mesh generation, materials, shaders, GPU pipeline |
| `opts-gui.log` | `opts_gui::*` | UI widget lifecycle, tokens, focus events, theme changes |
| `opts-game.log` | `opts_game::*` | Application state transitions, settings, main menu |
| `opts-tools.log` | `opts_tools::*`, `opts::*` | CLI conversion, inspection, and verification tools |
| `panics.log` | Fatal Panic Hook Output | Dedicated crash dump with thread name and backtrace |

---

## 📐 Output Formats

### 1. Interactive Console Format
Human-readable, colored when stdout is a TTY, compact timestamps:
```text
HH:MM:SS.mmm  LEVEL  target::module  message  {key=value}
```
**ANSI Color Badges:**
- `ERROR`: Bold Red
- `WARN`: Bold Yellow
- `INFO`: Green
- `DEBUG`: Blue
- `TRACE`: Dim / Magenta

### 2. Subsystem File Format
Precise UTC timestamps, full span hierarchy:
```text
YYYY-MM-DDTHH:MM:SS.mmmZ  LEVEL  target  [span_name{key=val}]  message  {field=val}
```

### 3. Structured JSON Format (Optional via Config)
For machine parsing and automated integration test harnesses:
```json
{"timestamp":"2026-10-08T20:15:00.123Z","level":"INFO","target":"opts_game::app_state","message":"State transition","fields":{"from":"Loading","to":"MainMenu"}}
```

---

## ⚙️ Configuration Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: Level,
    pub per_target: BTreeMap<String, Level>,
    pub console: ConsoleConfig,
    pub files: FilesConfig,
    pub format: LogFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleConfig {
    pub enabled: bool,
    pub ansi: AnsiMode,               // Auto | Always | Never
    pub verbosity: ConsoleVerbosity,  // Compact | Standard | Verbose
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesConfig {
    pub enabled: bool,
    pub directory: PathBuf,
    pub rotation: RotationPolicy,     // Daily | Hourly | Size(u64) | Never
    pub retention: RetentionPolicy,   // Days(u32) | Count(u32) | Forever
    pub per_subsystem: bool,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnsiMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsoleVerbosity {
    Compact,
    Standard,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotationPolicy {
    Daily,
    Hourly,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    Days(u32),
    Count(u32),
    Forever,
}
```

---

## 🔌 Public API Design

```rust
// Primary Initialization (returns RAII guard holding non-blocking writers)
pub fn init(config: LogConfig) -> Result<LogGuard, LogError>;
pub fn init_default() -> Result<LogGuard, LogError>;
pub fn init_from_env() -> Result<LogGuard, LogError>;

// RAII Guard that flushes all background non-blocking writers on drop
pub struct LogGuard {
    _guards: Vec<tracing_appender::non_blocking::WorkerGuard>,
}

// Dynamic Runtime Reconfiguration
pub fn set_level(target: &str, level: Level) -> Result<(), LogError>;
pub fn set_global_level(level: Level);

// Crash Telemetry
pub fn install_panic_hook(log_dir: &std::path::Path);
```

---

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **`tracing` Ecosystem Standard** | `tracing` is the idiomatic diagnostic framework in the modern Rust and Bevy ecosystem. Standard library log macros (`info!`, `warn!`) bridge seamlessly into `tracing`. |
| **Non-blocking Worker Threads** | Disk operations can take tens of milliseconds on spinning disks or heavily loaded SSDs. Offloading writes to a background channel maintains a smooth 60 FPS simulation frame time. |
| **Subsystem Granularity** | Debugging vehicle physics or spline parsing in isolation is drastically simpler when viewing `opts-sim.log` or `opts-core.log` without thousands of mesh loader events. |
| **Crash Survivability** | Fatal panics in release builds often vanish silently; capturing backtraces into `panics.log` and immediately flushing workers ensures field crash reports are preserved. |

---

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **Standard `log` crate only** | Lacks structured fields, spans, parent-child context, and composable subscriber layers. |
| **Direct `bevy::log::LogPlugin`** | Couples logging to the Bevy engine. Headless unit tests in `opts-core` and CLI tools in `opts-tools` cannot use Bevy. |
| **Monolithic Single Log File** | Heavy rendering and coordinate conversion logs drown out simulation and UI events, making post-test triage tedious. |
| **Blocking synchronous file writes** | Causes perceptible frame hitching whenever multiple debug events are written during asset streaming or map chunk loading. |

---

## 🌊 Consequences

### ✅ Positive
- **Deterministic and Non-intrusive:** 0% frame time impact from log I/O.
- **Engine Agnostic:** All crates (`opts-core`, `opts-sim`, `opts-tools`, `opts-game`) share the same unified diagnostic infrastructure.
- **Targeted Troubleshooting:** Dedicated log files provide clean, focused traces per subsystem.
- **Post-Mortem Reliability:** Panic hook ensures crashes leave an indelible breadcrumb trail in `panics.log`.

### ⚠️ Negative
- Adds `tracing-appender` background worker threads to process runtime.
- Applications must hold `LogGuard` until termination to guarantee non-blocking buffer flush.

### 🔄 Neutral
- Log files require rotation and retention pruning on application startup to bound disk consumption.

---

## 🔗 References

- [ADR-0001: Why Bevy and Rust](./0001-why-bevy-and-rust.md)
- [ADR-0002: Clean-Room Policy](./0002-clean-room-policy.md)
- [ADR-0005: Workspace Crate Architecture](./0005-crate-architecture.md)
- [Coding Standards & Diagnostics](../contributing/standards.md)
