# 🚀 Application Wiring — Runnable `opts-game`

You are wiring the OPTS application together in
`crates/opts-game/src/main.rs` so that the user can run and review
the application end-to-end.

This is an **integration task**, not a feature task. You are not
building new systems. You are composing the existing crates into a
coherent, launchable application.

**Success is defined as:** `cargo run -p opts-game` (or
`cargo run`) launches a working window that shows the main menu,
initializes logging, discovers the OMSI installation in the
background, and exits cleanly.

If a subsystem is a stub, wire it anyway — with graceful
degradation — so the app runs. Do not block the whole application
on one incomplete crate.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Primary target
`crates/opts-game/src/main.rs`

### Supporting files
- `crates/opts-game/src/lib.rs` — library entry (if it exists)
- `crates/opts-game/src/app_state.rs` — if state machine exists
- `crates/opts-game/Cargo.toml` — dependencies
- Root `Cargo.toml` — workspace members

### Output targets
- `crates/opts-game/src/main.rs` — the wiring
- `crates/opts-game/src/cli.rs` — CLI argument parsing
- `crates/opts-game/src/app.rs` — the `build_app()` function
- `crates/opts-game/src/shutdown.rs` — clean shutdown handler
- `docs/crates/opts-game.md` — updated with the wiring sequence
- `docs/src/architecture/application-startup.md` — new doc

### Related context (read first)
- `docs/src/architecture/main-menu.md` — the menu design
- `docs/src/architecture/crate-graph.md` — if it exists
- `docs/src/adr/0001-why-bevy-and-rust.md`
- `docs/src/adr/0004-logging-architecture.md` — if it exists
- `docs/src/contributing/standards.md`
- `crates/opts-game/Cargo.toml`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not add features to crates. If a crate needs a plugin it
  doesn't have, add a *stub* plugin in `opts-game` and note it.
- Do not hardcode absolute paths. Use `directories` or the workspace
  config.
- Do not use `println!` for logging. Use `opts-logging` /
  `tracing`.
- Do not block application startup on background tasks
  (discovery, scanning). The window must appear immediately.
- Do not panic on missing optional resources. Log and degrade.
- Do not commit any OMSI asset.

### ✅ Permitted

- Read every crate in the workspace.
- Add `opts-game` dependencies via the workspace `Cargo.toml`.
- Create the CLI, app builder, and shutdown modules.
- Register plugins from existing crates.
- Add stub plugins for subsystems that don't exist yet.
- Configure the Bevy window (title, size, mode).
- Handle CLI arguments for development convenience.
- Write integration tests that build the app headlessly.

---

## 3. Preconditions

Verify and report before starting:

1. `cargo check --workspace` passes.
2. `crates/opts-game/` exists and is a workspace member.
3. `crates/opts-game/src/main.rs` exists (even if stubbed).
4. The main menu example runs:
   `cargo run -p opts-game --example main_menu`.
5. `opts-logging` exists (or log crate placeholder exists).
6. `opts-gui` exists.

If any precondition fails, STOP and report. Do not proceed with a
broken baseline.

**Before writing any code, run this discovery step:**

For each of these crates, report whether it exists and what public
plugin or init function it exposes:

| Crate | Exists? | Public entry point (plugin / init fn) |
| :--- | :---: | :--- |
| `opts-logging` | ? | ? |
| `opts-core` | ? | ? |
| `opts-discovery` | ? | ? |
| `opts-sim` | ? | ? |
| `opts-bevy` | ? | ? |
| `opts-gui` | ? | ? |
| `opts-game` | ? | ? |

Use this table to determine what can be wired now and what needs a
stub. Report the table before proceeding to Part A.

---

## 4. Part A — Wiring Design

Before writing code, produce `docs/src/architecture/application-startup.md`
describing the startup sequence.

### Stage A1 — Startup Order

Define the exact order of operations at application startup:

```
1. Parse CLI arguments (clap)
2. Initialize logging (opts-logging)
3. Install panic hook
4. Configure window parameters
5. Build Bevy App
6. Add DefaultPlugins (or curated subset)
7. Register AppState
8. Add OptsLoggingPlugin (if it exists)
9. Add OptsBevyPlugin
10. Add OptsGuiPlugin
11. Add MainMenuPlugin
12. Add stub plugins for incomplete subsystems
13. Register startup systems
14. Run the app
15. On exit: flush logging, save settings
```

Adjust the order if the actual crates require it. Document the
reason for any deviation.

### Stage A2 — CLI Arguments

Define the CLI surface. Required arguments:

| Flag | Purpose | Default |
| :--- | :--- | :--- |
| `--map-path <PATH>` | Override the recent map path (for review) | from settings |
| `--log-level <LEVEL>` | Override log level (trace/debug/info/warn/error) | `info` |
| `--log-file <PATH>` | Override log directory | from config |
| `--windowed` / `--fullscreen` | Window mode | windowed |
| `--window-size <W>x<H>` | Initial window size | 1920x1080 |
| `--headless` | Run without a window (for CI tests) | false |
| `--recent-map <PATH>` | Persist this path as the last-loaded map | none |

Use `clap` with derive. Every flag must be documented with `///`
docs so `--help` is useful.

### Stage A3 — Window Configuration

Define the window settings:

- Title: `"OPTS — Open Public Transport Simulator"`
- Initial size: 1280×720 (or from `--window-size`)
- Resizable: yes
- Present mode: `AutoVsync`
- Minimum size: 800×600
- Exit on close: yes

Icon: if `opts-game/assets/icon.png` exists, use it. Otherwise, the
default Bevy icon.

### Stage A4 — Plugin Composition

For each plugin, define:

- Which crate provides it
- What resource it requires at registration
- Whether it needs a `Resource` initialized first
- What happens if the crate is a stub (stub plugin)

If a crate is a stub, the stub plugin should:
- Log a `WARN` at startup: "opts-discovery is not yet implemented"
- Insert any placeholder resources with sensible defaults
- Do nothing else

### Stage A5 — Startup Systems

Define the startup systems and their order:

1. **Load settings** (`GameSettings::load()`)
2. **Initialize the catalogue** (background task, non-blocking)
3. **Load the map background** (if `--map-path` or settings has one)
4. **Transition to `MainMenu`** after the first frame

The window must appear before the catalogue scan starts. The
background image load is asynchronous.

### Stage A6 — Shutdown

On application exit:

1. Save `GameSettings`.
2. Flush the `LogGuard`.
3. Log a final `INFO` event: "Application exited cleanly".

If the app exits via panic, the panic hook flushes logs.

### Stage A7 — Error Handling

Define the error strategy:

- **Top-level `main()`** returns `Result<(), anyhow::Error>`.
- **CLI parse errors** → clap prints usage, exits with code 2.
- **Logging init errors** → print to stderr, exit with code 3.
- **Bevy app errors** → logged via tracing, exit with code 4.
- **Missing OMSI installation** → logged as `WARN`, app still runs
  with the catalogue empty.

The app must **never** panic on missing optional resources.

### Stage A8 — Stub Plugins

For every crate that doesn't exist yet or is a stub, create a stub
plugin in `crates/opts-game/src/stubs/`:

```
crates/opts-game/src/stubs/
├── mod.rs
├── discovery.rs      # Stub for opts-discovery
├── sim.rs            # Stub for opts-sim (if stub)
└── ...
```

Each stub plugin:

1. Logs a `WARN` at startup: "stub: opts-X is not yet implemented".
2. Inserts any placeholder resources the rest of the app expects.
3. Does nothing else.

Stub plugins must be clearly marked and easy to remove when the
real crate is ready.

### Stage A9 — Checkpoint

Report the wiring design.

**Wait for confirmation before Part B.**

---

## 5. Part B — Implementation

### Stage B1 — CLI Module

Create `crates/opts-game/src/cli.rs`:

- `#[derive(Parser)]` struct `CliArgs`
- Every flag documented
- `CliArgs::parse_args()` helper
- Unit test that parses a representative command line

### Stage B2 — App Builder

Create `crates/opts-game/src/app.rs`:

- `pub fn build_app(args: CliArgs) -> App`
- Returns a fully configured Bevy `App` (not yet run)
- Registers all plugins, states, resources
- Used by both `main.rs` and integration tests

This separation matters: tests can build the app headlessly
without spawning a window.

### Stage B3 — Shutdown Module

Create `crates/opts-game/src/shutdown.rs`:

- `ShutdownPlugin` that reads `AppExit` events
- Saves settings, flushes logs before Bevy exits

### Stage B4 — `main.rs`

```rust
fn main() -> Result<(), anyhow::Error> {
    let args = CliArgs::parse_args();
    let _guard = opts_logging::init_from_args(&args)?;
    tracing::info!("OPTS starting");
    let mut app = build_app(args);
    app.run();
    Ok(())
}
```

`main.rs` should be **under 30 lines**. All logic lives in
`build_app` and the plugin modules.

### Stage B5 — Stub Plugins

Implement the stubs from Stage A8. Each stub is under 40 lines.

### Stage B6 — Verify Runnable

Run:

```bash
cargo run -p opts-game
```

The window should appear with the main menu. Logs should appear in
the console and in `~/.local/share/opts/logs/`.

Test the CLI:

```bash
cargo run -p opts-game -- --help
cargo run -p opts-game -- --log-level debug
cargo run -p opts-game -- --map-path /path/to/a/map
cargo run -p opts-game -- --headless
```

Each should behave as documented.

### Stage B7 — Headless Test

Create `crates/opts-game/tests/startup.rs`:

- Build the app with `--headless`.
- Run for 5 frames.
- Assert the `AppState` transitions from `Loading` to `MainMenu`.
- Assert no panics occurred.

### Stage B8 — Checkpoint

Report the implementation and runtime verification.

**Wait for confirmation before Part C.**

---

## 6. Part C — Documentation

### Stage C1 — `docs/src/architecture/application-startup.md`

Document:

- The startup sequence (from Stage A1)
- The CLI surface
- The plugin composition
- The shutdown sequence
- The stub pattern (and how to remove a stub when the real crate
  lands)

### Stage C2 — `docs/crates/opts-game.md`

Update to cover:

- The `main.rs` structure (or link to `app.rs`)
- How to add a new plugin
- How the CLI is parsed
- How stubs are used during development

### Stage C3 — `docs/src/SUMMARY.md`

Add `docs/src/architecture/application-startup.md` under Architecture.

### Stage C4 — `README.md`

If the README has a "Getting Started" section, update it with:

```
cargo run -p opts-game
```

And note any prerequisites.

### Stage C5 — Final Report

Report:

- The startup sequence as implemented
- Which crates are wired vs stubbed
- Runtime verification results (which `cargo run` commands worked)
- Any deviations from the design
- Files created and modified
- Recommended next steps

**This is the end of the task.**

---

## 7. Reporting Cadence

1. Precondition discovery — approval before Part A
2. Part A (design) — approval before Part B
3. Part B (implementation) — approval before Part C
4. Part C (documentation) — end of task

Format:

```
## Application Wiring — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Runtime status:** <does the app launch?>
**Crates wired:** <list>
**Crates stubbed:** <list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 8. Exit Criteria

- ✅ `cargo run -p opts-game` launches a window with the main menu.
- ✅ Logging is initialized and writes to console and files.
- ✅ CLI arguments work as documented (`--help` shows all).
- ✅ `--headless` runs the app without a window.
- ✅ `main.rs` is under 30 lines.
- ✅ All application logic lives in `app.rs`, `cli.rs`, `shutdown.rs`.
- ✅ Stub plugins exist for every incomplete subsystem.
- ✅ The window appears before background work begins.
- ✅ The app exits cleanly (settings saved, logs flushed).
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Startup integration test passes.
- ✅ `docs/src/architecture/application-startup.md` exists.
- ✅ `docs/crates/opts-game.md` updated.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No `println!` in library code.
- ✅ No panics on missing optional resources.

When finished, respond with the summary from Stage C5.

Then respond with the exact string:
"APPLICATION WIRED — ready for user review"