# 🏗️ Workspace Modularization & `opts-logging`

You are performing two connected tasks:

1. **Modularize the workspace.** The current crate structure has
   accumulated stub crates and mixed concerns. Redistribute code
   into a coherent, well-bounded crate graph.
2. **Implement `opts-logging`.** A new foundational crate providing
   the logging infrastructure for the entire project — console
   output plus per-subsystem log files.

These are connected because `opts-logging` is a new crate in the
target architecture, and its design influences how other crates are
restructured.

This is a MULTI-PHASE task. Each phase has a checkpoint. Do not
proceed between phases without user approval.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output targets

- `crates/opts-logging/` — the new crate
- `docs/src/adr/0004-logging-architecture.md` — ADR for logging
- `docs/src/adr/0005-crate-architecture.md` — ADR for the crate graph
- `docs/src/architecture/crate-graph.md` — the reference diagram
- `docs/crates/*.md` — updated per crate
- `Cargo.toml` — updated workspace members
- `docs/src/SUMMARY.md` — updated references

### Related context (read first)
- `docs/src/adr/0001-why-bevy-and-rust.md`
- `docs/src/adr/0002-clean-room-policy.md`
- `docs/src/contributing/standards.md`
- `docs/src/meta/critical-review-*.md` (the most recent review —
  contains findings about stub crates and inert dependencies)
- `CONTRIBUTING.md`
- `dev/BOARD.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not change the public API of any parser in `opts-core`. The
  parsers are stable; only the crate housing them may move.
- Do not break the boundary rules:
  - `opts-core` must not depend on Bevy.
  - `opts-sim` must not depend on Bevy.
  - `opts-logging` must not depend on any OPTS crate.
  - Only `opts-bevy`, `opts-gui`, and `opts-game` may depend on Bevy.
- Do not delete code. Move it, or mark it `#[deprecated]` with a
  migration note. Deletions require explicit user approval.
- Do not add `unsafe`.
- Do not add `unwrap()` / `expect()` / `panic!()` in library code.
- Do not use `println!` / `eprintln!` for logging anywhere. All
  diagnostics go through `tracing`.
- Do not create a crate per module. The target architecture should
  have as few crates as possible while respecting the boundaries.

### ✅ Permitted

- Read every file in the repository.
- Run `cargo check`, `cargo clippy`, `cargo test`, `cargo tree`,
  `cargo metadata`.
- Move source files between crates.
- Update `Cargo.toml` files (workspace and per-crate).
- Add new dependencies to the workspace dependency table.
- Create `opts-logging`.
- Produce ADRs and architecture documentation.

---

## 3. Preconditions

Verify and report before starting:

1. The workspace compiles cleanly (`cargo check --workspace`).
2. `cargo test --workspace` passes.
3. The most recent critical review report exists under
   `docs/src/meta/`.
4. ADR-0001 and ADR-0002 exist.
5. The `docs/src/contributing/standards.md` boundary rules are
   present.

If any precondition fails, STOP and report.

---

## 4. Phase A — Crate Boundary Analysis

Phase A produces a proposal for the target crate architecture.
**No code moves in this phase.**

### Stage A1 — Current Inventory

Enumerate every crate in the workspace. For each:

| Crate | LOC | Public items | Deps (internal) | Deps (external) | Tests | Purpose stated | Purpose actual |
| :--- | ---: | ---: | :--- | :--- | ---: | :--- | :--- |

"Purpose actual" is your best inference from reading the code. It
often differs from the stated purpose.

### Stage A2 — Boundary Violations

Identify every crate that violates a stated boundary:

- Crate depends on Bevy but isn't `opts-bevy`, `opts-gui`, or
  `opts-game`.
- `opts-core` or `opts-sim` performs I/O.
- A crate has dependencies that don't match its stated purpose.
- Two crates have overlapping responsibility.

For each violation, cite the file and line.

### Stage A3 — Stub Audit

For every crate with fewer than 100 LOC:

1. State its intended purpose from `Cargo.toml` and any doc comments.
2. State whether that purpose is:
   - **Still needed** — the crate should be kept and populated
   - **Redundant** — the purpose overlaps with another crate
   - **Premature** — the purpose is real but no code exists yet
   - **Orphaned** — the purpose is no longer relevant

### Stage A4 — Target Architecture Proposal

Propose the target crate graph. Requirements:

- **As few crates as possible.** Every crate must have a distinct,
  non-overlapping purpose. If two crates could be merged without
  violating boundaries, they should be.
- **Clear layering.** Foundational crates at the bottom, application
  crates at the top. No cycles.
- **Respects the boundaries.** `opts-core` and `opts-sim` have no
  Bevy dependency. `opts-logging` has no OPTS dependency.
- **Future-proof.** The architecture should accommodate the known
  future work (script VM, audio engine, human assets, ego
  perspective, GUI screens).

Deliver the proposal as a Mermaid dependency graph plus a table:

| Crate | Purpose | Depends on | Depended by | Status |
| :--- | :--- | :--- | :--- | :--- |

Where "Status" is one of: `existing`, `migrate`, `new`, `merge-into-X`,
`retire`.

### Stage A5 — Migration Plan

For each crate affected by the proposal:

| Source crate | Target crate | Files to move | Estimated LOC | Complexity |
| :--- | :--- | :--- | ---: | :--- |

Complexity is `low` (pure move), `medium` (move + adjust imports),
`high` (move + refactor + boundary adjustment).

Also list:
- Which crates get their `Cargo.toml` rewritten
- Which crates get retired
- Which crates get created
- The order of migration (bottom-up)

### Stage A6 — ADR-0005 Draft

Draft `docs/src/adr/0005-crate-architecture.md` capturing the
target architecture decision. Follow the ADR template. Include:

- Context (why the current structure is problematic)
- Decision (the target graph)
- Rationale (why these crates, not others)
- Alternatives considered (fewer crates, more crates, single crate)
- Consequences (positive and negative)

### Stage A7 — Checkpoint

Report the target architecture, migration plan, and ADR draft.

**Wait for user approval before Phase B.**

---

## 5. Phase B — Logging Design

Phase B produces the design for `opts-logging`. **No code yet.**

### Stage B1 — Requirements

Enumerate the logging requirements:

1. **Console output** — human-readable, colored, with target and
   level visible.
2. **File output** — multiple files, one per subsystem.
3. **Per-target filtering** — different levels for different
   modules (e.g., `wgpu=warn`, `opts_core=debug`).
4. **Runtime reconfiguration** — levels can change without restart
   (via settings reload or in-game menu).
5. **Non-blocking writes** — logging must not stall the game loop.
6. **Rotation** — daily or size-based.
7. **Retention** — keep N days, delete older.
8. **Panic integration** — flush writers on panic, write a crash
   marker.
9. **Bevy integration** — `opts-logging` initializes the global
   tracing subscriber before Bevy does; Bevy inherits it.
10. **Config sources** — file (default), env var (`RUST_LOG`),
    CLI override.

Add or remove items based on the codebase's needs.

### Stage B2 — Subsystem Routing

Design the per-subsystem file layout. Which subsystems get their
own file? Proposed default:

| File | Receives events from |
| :--- | :--- |
| `opts.log` | Everything (aggregate) |
| `opts-core.log` | `opts_core::*` |
| `opts-sim.log` | `opts_sim::*` |
| `opts-bevy.log` | `opts_bevy::*`, `wgpu`, `naga`, `bevy_render` |
| `opts-gui.log` | `opts_gui::*` |
| `opts-game.log` | `opts_game::*` |
| `opts-discovery.log` | `opts_discovery::*` |
| `panics.log` | Panic hook output |

Confirm or adjust. Every subsystem with more than ~500 LOC of code
probably warrants its own file. Subsystems smaller than that can
share.

### Stage B3 — Format Design

Define two formats:

**Console format:**

```
HH:MM:SS.mmm  LEVEL  target::module  message  {fields}
```

With ANSI colors:
- ERROR: red
- WARN: yellow
- INFO: green
- DEBUG: blue
- TRACE: dim

**File format:**

```
YYYY-MM-DD HH:MM:SS.mmmZ  LEVEL  target  [span1 span2]  message  {fields}
```

Optionally support JSON for tooling integration. If JSON is
included, specify the schema.

### Stage B4 — Config Schema

Design the `LogConfig` struct. Required fields:

```rust
pub struct LogConfig {
    pub level: Level,
    pub per_target: BTreeMap<String, Level>,
    pub console: ConsoleConfig,
    pub files: FilesConfig,
    pub format: LogFormat,  // Text | Json
}

pub struct ConsoleConfig {
    pub enabled: bool,
    pub ansi: AnsiMode,     // Auto | Always | Never
    pub verbosity: ConsoleVerbosity,  // Compact | Standard | Verbose
}

pub struct FilesConfig {
    pub enabled: bool,
    pub directory: PathBuf,
    pub rotation: RotationPolicy,   // Daily | Hourly | Size(u64) | Never
    pub retention: RetentionPolicy, // Days(u32) | Count(u32) | Forever
    pub per_subsystem: bool,
    pub json: bool,
}
```

Design should support loading from:
- TOML file
- Environment variables (`OPTS_LOG_LEVEL`, `RUST_LOG`)
- Programmatic API (`LogConfig::default()`, builders)

### Stage B5 — Public API

Design the public API of `opts-logging`:

```rust
// Initialization
pub fn init(config: LogConfig) -> Result<LogGuard, LogError>;
pub fn init_default() -> Result<LogGuard, LogError>;
pub fn init_from_env() -> Result<LogGuard, LogError>;

// The guard holds non-blocking writers. Dropping it flushes.
pub struct LogGuard { /* ... */ }

// Runtime reconfiguration
pub fn set_level(target: &str, level: Level) -> Result<(), LogError>;
pub fn set_global_level(level: Level);

// Panic integration (called automatically by init)
pub fn install_panic_hook();
```

`LogGuard` must be held by `opts-game` for the lifetime of the
application. It flushes all writers on drop.

### Stage B6 — ADR-0004 Draft

Draft `docs/src/adr/0004-logging-architecture.md` capturing the
logging decisions. Follow the ADR template.

Include:
- Context (why logging needs its own crate)
- Decision (the architecture, per-subsystem files, non-blocking)
- Rationale (why `tracing` + `tracing-subscriber` + `tracing-appender`)
- Alternatives considered (log crate, slog, custom)
- Consequences (positive and negative)

### Stage B7 — Checkpoint

Report the design (requirements, routing, format, config, API,
ADR draft).

**Wait for user approval before Phase C.**

---

## 6. Phase C — Logging Implementation

### Stage C1 — Crate Setup

Create `crates/opts-logging/`:

```
crates/opts-logging/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs                    # Public API + crate docs
│   ├── config.rs                 # LogConfig and friends
│   ├── init.rs                   # init(), init_default(), init_from_env()
│   ├── guard.rs                  # LogGuard
│   ├── error.rs                  # LogError
│   ├── layer/
│   │   ├── mod.rs
│   │   ├── console.rs            # Console formatting + colors
│   │   ├── file.rs               # File formatting
│   │   └── router.rs             # Per-subsystem routing layer
│   ├── rotation.rs               # Rotation & retention
│   ├── panic.rs                  # Panic hook
│   └── prelude.rs                # Re-exports
└── tests/
    ├── config_test.rs
    ├── init_test.rs
    ├── routing_test.rs
    └── fixtures/
        └── sample_config.toml
```

### Stage C2 — Dependencies

Add to `crates/opts-logging/Cargo.toml`:

```toml
[dependencies]
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = [
    "env-filter", "fmt", "ansi", "json", "registry", "std"
] }
tracing-appender = { workspace = true }
directories = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true, features = ["derive"] }
toml = { workspace = true }
time = { workspace = true, features = ["formatting", "macros"] }
nu-ansi-term = { workspace = true }
```

Add new workspace dependencies as needed. Justify each.

### Stage C3 — Config Module

Implement `LogConfig`, `ConsoleConfig`, `FilesConfig`, `LogFormat`,
`RotationPolicy`, `RetentionPolicy`, `AnsiMode`, `ConsoleVerbosity`.

Requirements:
- All types implement `serde::Deserialize` and `Default`.
- `LogConfig::from_file(path) -> Result<LogConfig, LogError>`.
- `LogConfig::from_env() -> LogConfig` merges `OPTS_LOG_*` vars.
- `LogConfig::validate() -> Result<(), LogError>` catches invalid
  combinations (e.g., retention < rotation interval).

### Stage C4 — Console Layer

Implement the console layer with ANSI colors:

- Respect `AnsiMode::Auto` — disable ANSI when stdout is not a TTY.
- Colored level badges (`ERROR` in red, etc.).
- Compact timestamps (`HH:MM:SS.mmm`).
- Target shown in dim color.
- Span context shown inline when present.

Provide three verbosity levels:
- **Compact**: one line per event, minimal.
- **Standard**: default, includes target and span context.
- **Verbose**: includes file and line.

### Stage C5 — File Layer

Implement the file layer:

- Text format with full timestamps and target.
- Optional JSON format.
- Rotation via `tracing-appender::rolling`.
- Retention: enumerate old files on startup, delete those beyond
  policy.

### Stage C6 — Per-Subsystem Routing

Implement the routing layer:

- A `RouterLayer` that inspects `Metadata::target()` for each event.
- Routes events to one or more writers based on target prefix.
- Aggregate writer receives everything.
- Subsystem writers receive only their prefix.

Use `tracing_subscriber::filter::Targets` for the filtering.

### Stage C7 — Panic Hook

Install a panic hook during `init()`:

- Capture the panic message and backtrace.
- Log an `ERROR` event to the `panics.log` file.
- Flush all writers.
- Call the original panic hook so the process still exits with a
  proper panic.

### Stage C8 — Public API

Implement `init`, `init_default`, `init_from_env`, `LogGuard`,
`set_level`, `set_global_level`, `install_panic_hook`.

`init` must:
1. Validate the config.
2. Set up the global `tracing` subscriber.
3. Install the panic hook.
4. Return a `LogGuard` holding all writers.

Calling `init` twice must return an error (`LogError::AlreadyInitialized`).

### Stage C9 — Tests

Required tests:

- **Config:** parse TOML, parse env, invalid config rejected.
- **Init:** init succeeds once, fails twice.
- **Routing:** events for `opts_core` appear in `opts-core.log`
  and `opts.log`, but not in `opts-sim.log`.
- **Format:** console output contains level, target, message.
- **Panic:** panics are logged and the process still panics.
- **Guard:** dropping the guard flushes writers.

### Stage C10 — Documentation

- `crates/opts-logging/src/lib.rs` — crate docs with examples.
- `docs/crates/opts-logging.md` — user-facing crate doc covering
  the public API, config format, and integration steps.
- `docs/src/adr/0004-logging-architecture.md` — promoted from
  draft to accepted.
- Reference in `docs/src/SUMMARY.md`.

### Stage C11 — Integration

Update `opts-game`:

- Depend on `opts-logging`.
- Initialize logging in `main.rs` before any other setup.
- Hold the `LogGuard` in a resource or thread-local.

If `opts-tools` exists, do the same for it.

### Stage C12 — Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p opts-logging
cargo run -p opts-game --example gallery  # or whatever the example is
```

Confirm:
- Log files appear in `~/.local/share/opts/logs/`.
- Console output is colored when run interactively.
- Console output has no ANSI codes when piped to a file.

### Stage C13 — Checkpoint

Report the implementation and runtime verification.

**Wait for user approval before Phase D.**

---

## 7. Phase D — Migration

Phase D migrates code between crates per the Phase A plan.

### Stage D1 — Order

Migration is done **bottom-up**: leaf crates first. From the
Phase A plan, execute migrations in the following order:

1. Create `opts-logging` (done in Phase C).
2. Migrate the lowest-layer crates.
3. Migrate middle-layer crates.
4. Migrate application-layer crates.
5. Retire crates marked for retirement.

### Stage D2 — Per-Crate Migration

For each migration:

1. Announce the migration (which crate, which files, which LOC).
2. Move files with `git mv` to preserve history.
3. Update `use` paths in the moved files.
4. Update `use` paths in dependent crates.
5. Update `Cargo.toml` for both source and target crates.
6. Run `cargo check` for the affected crates.
7. Run `cargo test` for the affected crates.
8. Commit with a message like:
   `refactor(opts-core): move X to opts-Y (M-NNNN)`

Do NOT batch migrations. One crate at a time. One commit per
migration.

### Stage D3 — Retire Unused Crates

For each crate marked `retire` in the Phase A plan:

1. Confirm no workspace member depends on it.
2. Remove it from `Cargo.toml` workspace members.
3. Delete the directory.
4. Update any documentation that references it.

Deletions are destructive — require user confirmation per crate
via the checkpoint.

### Stage D4 — Update Workspace Cargo.toml

- Add `opts-logging` to members and dependencies.
- Remove retired crates from members.
- Update the dependency table.

### Stage D5 — Update Documentation

- `docs/src/architecture/crate-graph.md` — the reference diagram
  and table.
- `docs/src/adr/0005-crate-architecture.md` — promoted to accepted.
- `docs/src/SUMMARY.md` — add `opts-logging`, remove retired.
- `docs/crates/opts-logging.md` — already created in Phase C.
- `docs/crates/*.md` — update per-crate docs for affected crates.
- `CONTRIBUTING.md` — update the crate boundary rules if they
  changed.

### Stage D6 — Checkpoint

Report the migration completion:

- Crates moved
- Files moved
- Crates retired
- Commits made
- Any migrations that were deferred (with reason)

**Wait for user approval before Phase E.**

---

## 8. Phase E — Verification

### Stage E1 — Build and Test

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

All must pass.

### Stage E2 — Boundary Verification

Run:

```bash
cargo tree -p opts-core
cargo tree -p opts-sim
cargo tree -p opts-logging
```

Confirm:
- `opts-core` has no Bevy.
- `opts-sim` has no Bevy.
- `opts-logging` has no OPTS crates.

Document in the ADR-0005 that these are now enforced invariants.

### Stage E3 — Logging Runtime Verification

Run `opts-game` (or the example binary) and confirm:

- Log files appear in the configured directory.
- Per-subsystem files are populated.
- Console output is colored when interactive.
- No ANSI codes when piped.
- Panics are logged to `panics.log`.
- Log guard flushes on exit.

### Stage E4 — Final Report

Produce `docs/src/meta/modularization-YYYY-MM-DD.md`:

- Current crate count before/after
- LOC moved
- Crates created, retired, merged
- ADRs accepted
- Any boundary violations discovered (or confirmed fixed)
- Any follow-up items

---

## 9. Reporting Cadence

Checkpoints:

1. Phase A — approval before Phase B
2. Phase B — approval before Phase C
3. Phase C — approval before Phase D
4. Phase D — approval before Phase E
5. Phase E — final report

Format:

```
## Modularization — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Crates affected:** <list>
**Findings:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 10. Exit Criteria

- ✅ Target crate architecture defined in ADR-0005.
- ✅ Logging architecture defined in ADR-0004.
- ✅ `opts-logging` exists, is documented, and has tests passing.
- ✅ `opts-game` initializes logging at startup.
- ✅ Log files are written per subsystem.
- ✅ Console output is colored interactively, plain when piped.
- ✅ Panics are logged.
- ✅ All migrations from Phase A executed (or deferred with reason).
- ✅ Retired crates removed (with user approval).
- ✅ `cargo clippy -- -D warnings` passes.
- ✅ `cargo test --workspace` passes.
- ✅ `docs/src/SUMMARY.md` reflects the new structure.
- ✅ `docs/crates/*.md` updated for affected crates.
- ✅ No `unsafe`.
- ✅ No `println!` in library code.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.

When finished, respond with the summary from Stage E4.

Then respond with the exact string:
"MODULARIZATION & LOGGING COMPLETE — ready for P-001"