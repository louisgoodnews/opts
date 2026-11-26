# Research: opts-debugger, the development toolkit crate

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-096, stage 1 (research; no code) |
| **Date** | 2026-11-26 |
| **Base** | integration `74807277` |
| **Scope** | read-only inventory of the existing development and debugging tools, the gaps, a proposed first version, its placement and rules |
| **Decision owner** | the owner (ISS-096 "Decisions needed") |

Path convention: file references are repository-relative with line numbers where a specific claim was read.

---

## 1. Inventory of existing development and debugging tools

### 1.1 Tools by crate and kind

| Tool | Location | What it inspects | What it writes | Kind |
| :--- | :--- | :--- | :--- | :--- |
| Logging crate | `crates/opts-logging/src/lib.rs:1-16` | Tracing events, routed by target to subsystem files | `opts.log`, one file per subsystem, `panics.log` (`src/panic.rs:24`) | library |
| Log env switches | `crates/opts-logging/src/config.rs:65-91` | Reads `OPTS_LOG_LEVEL` (or `RUST_LOG`), `OPTS_LOG_CONSOLE`, `OPTS_LOG_DIR`, `OPTS_LOG_FORMAT` | none (configuration) | library |
| Default log folder | `crates/opts-logging/src/config/sinks.rs:66-68` | Per-user data folder `logs` (private fn `default_log_dir`) | none | library |
| System timing layer | `crates/opts-logging/src/layer/timing.rs:1-5, 47-48` | Busy time per Bevy `system` span over a window, heaviest N; enabled by `OPTS_SYSTEM_TIMING=<seconds>` | windowed summary lines through `tracing` (the log) | library |
| Stream trace | `crates/opts-logging/src/stream_trace.rs:1-8`; `crates/opts-game/src/in_game/session.rs:103-112` | Tile-streaming spans per frame (feature `stream-trace`) | one JSON line per frame to `OPTS_STREAM_TRACE=<file>` | library + game |
| Per-frame CSV | `crates/opts-game/src/in_game/session.rs:133-144` | Real time, frame time, position, speed, fixed steps | one CSV line per frame to `OPTS_DBG_FRAMES=<file>` | game (inside the binary) |
| Reflection frame stats | `crates/opts-bevy/src/reflections/stats.rs:1-30`; `crates/opts-bevy/src/reflections.rs:21-22` | Frame-time summary after a warm-up (`OPTS_REFLECT_STATS_AFTER`, default 20 s) | summary file to `OPTS_REFLECT_STATS=<file>`, then the app exits | Bevy plugin |
| Render stats | `crates/opts-bevy-instancing/src/render_stats.rs:1, 16, 50, 65, 110`; `crates/opts-game/src/control.rs:40` | Frame times (average, median, p99, 1 % low), instancing mode; `OPTS_RENDER_DIAG=1` adds GPU time per pass | answer of `render.stats` | Bevy plugin + control method |
| Control server | `crates/opts-bevy/src/control/` (`server.rs`, `server/dispatch.rs`, `handlers/`, `channel.rs`) | Live state over JSON-RPC 2.0 on TCP; token from `OPTS_CLI_TOKEN` (`control/server.rs:86`) | replies; screenshots and reports into the reports folder | library (inside `opts-bevy`) |
| Game control methods | `crates/opts-game/src/control_drive.rs:46` (`snapshot`), `control_trace.rs:19` (`trace.*`), `control_traffic.rs:22` (`traffic.dump`), `control.rs:40` (`render.stats`), plus `control_session.rs`, `control_*` | Session, scene trace, traffic, drive, settings, language | replies | library (inside `opts-game`) |
| Protocol spec | `docs/src/architecture/control-protocol.md` §4.1-4.14 (command surface), §4.2 lines 85-100 (queries) | The contract of the methods above | none | doc |
| Control client and CLI | `crates/opts-cli/src/lib.rs:1-12`, `src/client.rs`, `src/transport.rs`, `src/repl.rs`; binary `opts-cli` (`crates/opts-cli/Cargo.toml:19-22`) | Remote control of a running game: a command table (`src/commands/table/*.rs`), a REPL, JSON output | stdout | both: library (protocol types, command table, client behind feature `client`) and binary |
| CLI examples | `crates/opts-cli/examples/` (`drive_and_screenshot.rs`, `reload_iterate.rs`, `scene_probe.rs`, `agent_session.sh`, `play_session.sh`) | A driven session, screenshots, reload loops | screenshots, stdout | examples |
| Asset inspector | `crates/opts-tools/src/main.rs:1-12`; `crates/opts-tools/src/inspect.rs` | `opts-tools inspect <file>...`: OMSI files (`.o3d`, `.x`, `.sco`, `.sli`, `.oft`, `.terrain`, `.map`) through the real parsers | one line per file; exit code 1 on failure | binary (`opts-tools`, `Cargo.toml:14-16`) |
| Other `opts-tools` commands | `crates/opts-tools/src/main.rs:1-12`; `src/hardware.rs`, `src/l10n.rs`, `src/image_metrics.rs` | Hardware profile (`hardware report`), language packs (`l10n export/check/new/import/stats`), screenshot metrics (`image-metrics`) | stdout, JSON | binary |
| Benchmark binary | `crates/opts-benchmark/Cargo.toml`; `crates/opts-benchmark/src/scenery/worker.rs:109-113` (`OPTS_BENCH_INJECT` fault hook) | Scenery load verification, worker supervision | benchmark reports | binary |
| Scene trace | `crates/opts-scene-trace/src/lib.rs:1-4`; writer `src/store/writer.rs` | What is placed where: one record per placed thing, with mesh and decode notes | `scene-trace-<stamp>-<map>.jsonl` with a header line (`store/writer.rs`) | library, depends on `bevy` (`Cargo.toml:15`) |
| Probes (Bevy look-and-check) | `crates/opts-bevy/examples/reflections_probe.rs`, `light_cones_probe.rs`, `transmap_probe.rs`, `scenery_scripts.rs`; `crates/opts-game/examples/weather_probe.rs:7-9` (`OPTS_PROBE_HEIGHTS`, `OPTS_PROBE_PITCH`); `crates/opts-benchmark/examples/offscreen_probe.rs` | A scene rendered at chosen poses, with screenshots | screenshots, reports | examples |
| Dev aids (env) | `crates/opts-world-load/src/dev_look.rs:1-26` (`OPTS_DEV_LOOK_AT`); `crates/opts-game/src/lib.rs:121-124` (`OPTS_DEV_HOUR`); `crates/opts-game/src/in_game/loading_pipeline/stages/camera.rs:45-53` (`OPTS_DEV_CAM_HEIGHT`) | Start state of a session (object, hour, camera height) | none | game |
| Reports folder | `crates/opts-bevy/src/reporting/save.rs:19-21` | Screenshots and reports from the control server and F1 | `<data dir>/opts/reports` | library |
| `dbg!` calls | repository-wide grep | none | none | none in `crates/` (0 matches) |

Examples: about 110 `.rs` files under `examples/` in nine crates (`opts-bevy` 64, `opts-sim` 11, `opts-traffic` 10, `opts-gui` 9, `opts-benchmark` 6, `opts-cli` 4, `opts-game` 4, `opts-catalogue` 1, `opts-omsi-binary` 1). Most are Bevy programs that render and print. Examples count toward the 150-line rule (`docs/src/contributing/standards.md:639`).

### 1.2 Scripts under `scripts/`

Python (25 files; the eight `test_*.py` are the tests of their scripts):

| Script | One line |
| :--- | :--- |
| `bellows_census.py` | Census of the articulation bellows of every vehicle in an OMSI installation |
| `bench_compare.py` | Compares two Criterion runs of the M-0028 benches on one machine |
| `capture-fps-test.py` | Frame-rate cost of screen recording on a loaded scene |
| `check_links.py` | Fails when a relative Markdown link points at a missing file |
| `content_issues.py` | Builds the content issue registry from OPTS logs |
| `crate_graph.py` | Keeps `crate-graph.md` in step with the workspace (M-0074) |
| `dup_scan.py` | Duplication scan and unused-`pub` check (ISS-035, measured, not enforced) |
| `gen_roadmap_copy.py` | Regenerates the book's copies of `ROADMAP.md` |
| `mesh_issues.py` | Builds the mesh issue registry (P-1014) from the audit output |
| `omsi_event_census.py` | Census of the event sources in an installed OMSI data tree (M-0118) |
| `omsi_uses.py` | Ratchet for the ADR-0014 boundary: engine crates must not reach OMSI types |
| `roadmap_audit.py` | Roadmap drift audit (M-0074) across ROADMAP, phase pages, milestones, board |
| `setvars-check.py` | Screenshot recipe for the equipment options of a vehicle (M-0078) |
| `size_check.py` | Reports files over 150 code lines and functions over 80 (standards §3.7, §1.6) |
| `streaming-perf.py` | Tile-streaming frame-time benchmark (M-0089) |
| `structure_audit.py` | Structure audit: file and function sizes, inline tests, allows, crate budgets (M-0125, M-0147) |
| `uv_incident_stats.py` | Summarises the UV incident files of a log folder |
| `migration/bevy019_ui.py` | One-off codemod for the Bevy 0.14 to 0.19 UI bundle removals (M-0044); not a dev tool |
| `uv_truth_analysis.py` | Error statistics from the `vertices.tsv` of a folder |
| `wheel_census.py` | Census of wheel, steering and suspension animations of an OMSI installation |

Shell and PowerShell (9 files):

| Script | One line |
| :--- | :--- |
| `agent-worktree.sh` | Creates a worktree and branch `agent/<topic>` for one agent |
| `capture-agent-demo.sh` | Simulates an agent workflow (record, drive, screenshot) through the control protocol |
| `install-hooks.sh` | Points `core.hooksPath` at `scripts/` (pre-commit hook, M-0052) |
| `instancing-bench.sh` | Instancing benchmark on fixed poses through the control protocol (M-0082) |
| `local-ci.sh` / `local-ci.ps1` | Local gate: format, lint, tests, docs, mdBook |
| `shadow-cull-bench.sh` | Setting A/B benchmark through the control protocol (M-0029) |
| `visual-ab.sh`, `visual-shot.sh` | Deterministic screenshots for visual comparisons through the control protocol |

Other: `scripts/visual-views/m0122-reference.poses` (reference poses for the M-0122 measurement).

### 1.3 Crate kinds in short

- Both library and binary: `opts-cli` (client library and `opts-cli` binary), `opts-tools` (binary only), `opts-benchmark` (binary).
- Library inside a game binary: the control server (`opts-bevy`), the game methods (`opts-game`), the timing and stream layers (`opts-logging`).
- Bevy-free tool crates: `opts-logging`, `opts-cli`, and `opts-tools`, whose dependencies are `opts-core`, `opts-hardware` and `opts-l10n` (`crates/opts-tools/Cargo.toml:20-21`), none of which the graph marks as depending on Bevy. `opts-scene-trace` depends on Bevy (`crates/opts-scene-trace/Cargo.toml:15`).

---

## 2. Overlap and gaps

### 2.1 Overlap (do not rebuild)

| Need | Existing tool | Note |
| :--- | :--- | :--- |
| Live state of the session in one call | `snapshot` (`crates/opts-game/src/control_drive.rs:46`); `opts-cli` command in `crates/opts-cli/src/commands/table/scene_and_trace.rs:43` | The protocol's own "call an agent makes to see what happened" (`control-protocol.md` §4.13) |
| Remote control client with token auth | `opts-cli` (`crates/opts-cli/src/client.rs`, `src/token.rs`) | Used by the examples and by `opts-bevy` (`Cargo.toml` depends on `opts-cli` with `client`) |
| Asset validation of OMSI files | `opts-tools inspect` (`crates/opts-tools/src/inspect.rs`) | Per file, with the real parsers |
| Installation census | `omsi_event_census.py`, `wheel_census.py`, `bellows_census.py` | Per installation, in Python |
| Benchmark comparison | `bench_compare.py`; `streaming-perf.py`; `opts-benchmark` | Overlaps a future frame-time reader |
| Crate budgets and sizes | `structure_audit.py` (`check_crate_budget`, lines 503-514) | The measured source of the budgets |

### 2.2 Gaps, with the evidence for each

| # | Gap | Evidence of the need | Evidence that it is missing |
| :--- | :--- | :--- | :--- |
| G1 | **Frame-time report reader.** Frame times from `OPTS_DBG_FRAMES`, `OPTS_STREAM_TRACE` and `OPTS_REFLECT_STATS` are read by hand. | `docs/src/roadmap/phase-12/M-0122.md:111`: frame times "are the CSV frame deltas inside each pose window", computed in a scratch copy of `visual-ab.sh`. `docs/dev/BOARD.md:526-527`: p99, median and "1 % low" were reported for hitch investigations. `docs/src/architecture/performance-profiling.md:16-21` documents the CSV and the timing output as manual steps. | No reader in any crate or script; `scripts/streaming-perf.py` covers one benchmark only. |
| G2 | **Timing-log reader.** `OPTS_SYSTEM_TIMING` writes its windows into the log; ranking them is manual. | `docs/src/architecture/performance-profiling.md:21`: `OPTS_SYSTEM_TIMING=15 ... 2> timing.txt`. | No parser in `crates/opts-logging` or `scripts/`. |
| G3 | **`debug.stats` is documented but not built.** The protocol lists it as P2; M-0122 found it returned nothing and used `render.stats`. | `control-protocol.md` §4.6 (`debug.stats`); `docs/src/roadmap/phase-12/M-0122.md:111`. | `crates/opts-game/tests/control_protocol/commands.rs:7-18` lists `debug.stats` as not implemented; `opts-cli` still exposes it (`commands/table/reload_and_debug.rs:69`). |
| G4 | **Entity, resource and asset dump over the protocol.** | `control-protocol.md` §4.2 marks `entity.list`, `entity.get`, `resource.list`, `resource.get`, `asset.list`, `asset.get` as P2. The `opts-cli` table already lists them (`commands/table/basics.rs:80-85`). | `commands.rs:7-18` lists all six as not implemented; `crates/opts-bevy/tests/control.rs:109` calls `entity.list` in a test. This is server work in `opts-bevy` and `opts-game`, outside a toolkit crate. |
| G5 | **Scene-trace reader offline.** The trace writes one JSONL file per session; the only reader is the live `trace.*` method set, plus a test. | `crates/opts-scene-trace/src/store/writer.rs` writes the file with a header; `crates/opts-scene-trace/src/mode.rs:21` is the only parse function found outside the tests. `control-protocol.md` §4.12b describes the live queries. | No offline query of a saved trace file. |
| G6 | **One bundle per run.** Logs, panics, reports, the timing and frame files and the scene trace sit in different folders, and the user names some of them with environment switches. | Default log folder `sinks.rs:66-68`; reports folder `save.rs:19-21`; `panics.log` `panic.rs:24`; `scripts/visual-ab.sh` copies its own files into scratch (M-0122 method). | No collector. The log folder helper `default_log_dir` is private (`sinks.rs:66`), so a collector cannot reuse it without a change to `opts-logging`. |
| G7 | **OMSI file checker over a whole installation.** | `opts-tools inspect` checks one file at a time; census scripts count by folder. | Small gap: a single `check` over a folder is a convenience on top of `inspect`, not a new capability. Not proposed for the first version. |
| G8 | **Crate size check by hand.** Some budget estimates were made by awk or by hand. | `docs/dev/issues/ISS-086-epic-split-opts-sim-passengers.md:13` ("measured by an awk count"); `docs/dev/research/crate-splits-2026-11-26.md:33` (five of nine budgets were hand or awk estimates). | `structure_audit.py` already measures budgets; this is a process gap, not a debugging tool. Not proposed. |

### 2.3 Stale documentation found on the way

- `docs/src/architecture/control-protocol.md` §4.2 and §4.6 describe `entity.*`, `resource.*`, `asset.*` and `debug.*` as planned; the `opts-cli` table and the help text present them as commands. The built state is the list in `commands.rs:7-18`. Any tool that uses the help output must check that list.

---

## 3. Proposed scope: first version

Ordered by value. The first three need no Bevy and no server change. The fourth waits for a decision (open question 2).

| Tool | Input | Output | Reuses | Rationale |
| :--- | :--- | :--- | :--- | :--- |
| `frames` | A frame file: the `OPTS_DBG_FRAMES` CSV, the `OPTS_STREAM_TRACE` JSON lines or the `OPTS_REFLECT_STATS` summary; optional window markers (for example the pose windows of `visual-ab.sh`) | Count, mean, median, p95, p99, 1 % low, frames over 45 ms and 100 ms, the N worst frames with their frame numbers, one row per window. Plain text and `--json` | The line formats of `session.rs:103-144` and `reflections/stats.rs`. A copy of the parsing rules; no code shared at first | G1: M-0122 and the hitch rows of BOARD.md were computed by hand. Highest frequency of use in the record. |
| `timing` | A log file of a run started with `OPTS_SYSTEM_TIMING` | The heaviest systems per window, with total, runs and longest run | The line format written by `opts-logging/src/layer/timing.rs`, read from the same crate (a reader next to the writer) | G2: the documented workflow ends in a text file that nobody reads with a tool. |
| `bundle` | A run folder or the default folders, and the paths of any env-switch files | One folder or archive with a manifest: file list, sizes, the `OPTS_*` variables set when the game ran, the build from `version` when a server answers, and the logs and reports of that run. The token file and `OPTS_CLI_TOKEN` are never included | The default folders from `opts-logging` (needs the private `default_log_dir` made public, or a copy) and `opts-cli` for the `version` and `snapshot` calls | G6: the bug-report workflow collects the same set by hand. A manifest makes a report reproducible. |
| `trace` | A `scene-trace-*.jsonl` file; filters: `--find <text>`, `--tile x,y`, `--count` | Matching records as a table or JSON, and per-tile counts | The record types of `opts-scene-trace` (`src/types/record.rs`), which depend on Bevy through the crate; see section 4 | G5: the file has no reader outside the tests. Built second, after the Bevy-free split decision. |

Not in the first version, with the reason:

- An entity or resource dump (G4): the server methods do not exist (`commands.rs:7-18`). The work is in `opts-bevy` and `opts-game`. The toolkit can add a client for them once they exist.
- A session inspector: `snapshot` and `state` already answer it live (section 2.1). The offline part is `bundle`.
- An OMSI checker (G7): `opts-tools inspect` covers the file level; a folder check is a later extension of it.

---

## 4. Placement

### 4.1 Library, binary, or both

Proposal: **both**. A library (`opts-debugger`) holds the pure readers: `frames`, `timing`, `trace` parsing over `&str` or `&[u8]`, and the manifest model. A binary (`opts-debugger`) holds the commands, the file access and the output. Reason: the readers can then be tested on fixture text without the file system, as `docs/src/contributing/standards.md` §3.4 asks for parsers, and the binary can be replaced without touching them.

An alternative is to add the commands to the existing binary `opts-tools` (section 6, question 1).

### 4.2 Bevy

- `frames`, `timing` and `bundle` need no Bevy: they read text and files, and `opts-cli` is Bevy-free (`crates/opts-cli/Cargo.toml`, no internal dependencies; the graph lists it as "Zero Bevy", `crate-graph.md` Layer 0).
- `trace` needs the record types of `opts-scene-trace`, which depends on `bevy` (`crates/opts-scene-trace/Cargo.toml:15`) and uses Bevy math in `src/pick.rs:4`. The record module itself (`src/types/record.rs`) has no Bevy import. Two routes:
  - (a) Depend on `opts-bevy`, which already depends on `opts-scene-trace`. This is the route ISS-095 gives for a new crate that needs Bevy. No cycle: `opts-bevy` does not depend on `opts-debugger` (grep: no crate depends on it, and the name is free in every `Cargo.toml`).
  - (b) Move the record and parser into a Bevy-free crate, as ISS-092 did for the zones (`docs/dev/issues/ISS-095-opts-bevy-dependency-cycle.md`, "opts-spatial-zones (ISS-092)"). This keeps the toolkit Bevy-free.
- Recommendation: first version Bevy-free. Choose (b) for `trace` when it is built, or drop `trace` if the owner does not need offline reads.

### 4.3 Position in the crate graph

- Layer 3 (Application Executables & Tooling), next to `opts-tools` and `opts-benchmark`, which the graph places there (`docs/src/architecture/crate-graph.md`, Layer 3 subgraph).
- Dependencies: `opts-cli` (client, to call the server), and `opts-logging` only if `timing` reuses its line format. No dependency on `opts-game`, `opts-bevy` or any game crate.
- Dependents: none. Checked by grep: no `Cargo.toml` and no `.rs` file under `crates/` names `opts-debugger`; the only mentions are in `docs/dev/issues/ISS-096-epic-opts-debugger-toolkit.md`. The game's dependency graph is unchanged, because nothing in the game depends on the new node.
- The graph document needs a new node, a dependency edge set (none outgoing to game crates) and an inventory row. `scripts/crate_graph.py` is documented as the tool that keeps `crate-graph.md` in step with the workspace (its docstring); run it after the crate exists rather than editing the graph by hand.

### 4.4 Dependency cost

`opts-cli` with feature `client` brings `rustyline` and `directories` (`crates/opts-cli/Cargo.toml`, `[features]`). A toolkit that only reads files can use `opts-cli` without `client` for the types, and enable `client` for the commands that talk to a server.

---

## 5. Budget and rules

### 5.1 Budget

- Rule: `docs/dev/crate-budget.txt` must hold an entry for `opts-debugger`, set to its measured code-line total of `src/` once the skeleton exists (ISS-096 stage 3). `structure_audit.py` fails a crate that has no entry (`scripts/structure_audit.py:509-510`) and fails one above its entry (`:512-514`). Budgets may only shrink.
- No value is written now. The rule for the value: measured by `structure_audit.py`, not estimated by hand or awk (the lesson of ISS-086 and of `crate-splits-2026-11-26.md:33`).

### 5.2 Coding rules

| Rule | Applies | Why |
| :--- | :--- | :--- |
| §3.7 150-line file rule and §1.6 80-line function rule (`standards.md:634`, `:149`) | Yes, to `src/`, `tests/` and `examples/` (`standards.md:639`) | Same as every crate. |
| §3.14 file budgets and ratchet (`standards.md:801-808`) | Yes | The toolkit is read by agents too. |
| §2.1 no `unwrap`/`expect`/`panic!` in library code; `anyhow` in binaries with `.context` (`standards.md:234-253`) | Yes: library code is `Result` with `thiserror`; the binary uses `anyhow` as `opts-tools` does | Same split as `opts-tools` (binary) and `opts-logging` (library). |
| §2.9 `tracing`, not `println!` (`standards.md:389-400`) | Library: yes. Binary: `println!` only for actual user output (CLI tool results), as the rule says (`standards.md:392-393`) | A CLI prints its results; diagnostics still go through `tracing`. |
| §3.4 parsing is pure (`standards.md:562-572`) | Yes, for the readers. Its wording names `opts-core`; the same reasoning applies to the readers here | This is what lets the readers be tested on text. |
| §2.10 `unsafe` | Yes, unchanged | No reason to waive. |
| §2.11 crate boundaries: "Zero File I/O" tags | **Waived for the binary and the `bundle` reader**, because reading files is the purpose. Kept for the parse modules | The tag is a design statement in `crate-graph.md`, not a compile check; the readers keep it. |
| §2.11 "only `opts-bevy`, `opts-gui`, `opts-game` may depend on Bevy" (`standards.md:458`) | Applies if the crate depends on Bevy. Route through `opts-bevy` (ISS-095). The rule's amendment to name the exempt crates is still open (ISS-063 status row) | `opts-scene-trace` already depends on Bevy directly; that is an existing exception, not a precedent for this crate. |
| Test gate (the owner's full test gate: `cargo test --workspace --no-fail-fast`) | Yes, for the skeleton and each tool | Structure and clippy gates do not run tests. |

---

## 6. Risks and open questions for the owner

1. **New crate or the existing `opts-tools`?** `opts-tools` is already a Bevy-free dev binary with the same audience (`inspect`, `hardware`, `l10n`, `image-metrics`; `crates/opts-tools/src/main.rs:1-12`). Options: a new `opts-debugger` crate for the run-analysis tools (`frames`, `timing`, `bundle`, `trace`), or new subcommands of `opts-tools`. Recommendation: a new crate, because the run-analysis tools are a different audience (a player's log folder, a frame file) from the asset tools, and `opts-tools` has its own budget of 1,245 lines (`docs/dev/crate-budget.txt`).
2. **Bevy for `trace`.** Route (a) through `opts-bevy` (the crate then depends on Bevy) or route (b) by a Bevy-free split of the record schema (ISS-092 pattern). Or drop `trace` from the first version. Decide before the `trace` tool is started.
3. **Client ownership.** Reuse `opts-cli` (Bevy-free library, `client` feature) rather than owning a second client? Recommendation: reuse. This answers the ISS-096 question "reuse or own" for every tool that touches the control server.
4. **Entity and resource dump (G4).** The six `entity.*`, `resource.*` and `asset.*` methods are documented but not built, and building them is server work in `opts-bevy` and `opts-game`. Decide whether that server work is in scope for ISS-096 at all, or is a separate issue, before any client tool for it is planned.
5. **What a bundle may contain.** A bundle collects logs, reports and screenshots; logs can hold paths, and the reports folder holds screenshots of the player's session. Decide the default content (logs and frame files only, or screenshots too), and confirm that the control token (`OPTS_CLI_TOKEN`, `--cli-token-file`) is always excluded.
