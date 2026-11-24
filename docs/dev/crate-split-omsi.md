# Crate split: opts-omsi into five parser crates

Branch `agent/omsi-split`, from master `6fb458e7`. Purpose: bring `opts-omsi` (the legacy OMSI
readers, ADR-0014 leaf adapter) well below its size and keep every crate under 6,000 code lines,
without a behaviour change. The budgets in `docs/dev/crate-budget.txt` (ISS-034) are not raised:
each new crate gets an entry at its own measured size, and `opts-omsi` is lowered to its new total.

Reads: `docs/src/architecture/crate-graph.md` (opts-omsi is an L1 leaf adapter), ADR-0014 (the
legacy/native boundary), ADR-0002 (OMSI binaries are never inspected: the split touches source
only), and `docs/dev/crate-split-steam.md`, `crate-split-weather.md` (the pattern followed here).

## Measured before (`structure_audit.py check`, code lines in `crates/opts-omsi/src`)

Total **18,030** (budget 18,030). Largest modules: `formats/vehicles` 3,523 (incl. 34 test lines),
`formats/map` 2,575, `formats/scenery` 2,430, `formats/o3d` 1,869, `formats/chrono` 1,558,
`formats/x` 1,261, `formats/trains` 894, `formats/timetable` 806, `setvars` 582.
Integration tests in `crates/opts-omsi/tests` (3,048 lines) are outside the budget and stay on the
facade: they use public paths only (`opts_omsi::formats::...`).

## What moves

| New crate | Modules (files move with `git mv`, paths kept under the new `src/`) | Code lines |
| :--- | :--- | ---: |
| `opts-omsi-text` | `formats/text.rs` + `text/`; the text-only readers `constfile`, `varlist`, `envir`, `engine_config`, `cti`, `oft`, `org`, `money`, `humans`, `ticketpack`, `weather`, `timetable`, `sound.rs` + `sound/` | 3,290 |
| `opts-omsi-binary` | `formats/o3d/` and `formats/x/` (the binary mesh readers `.o3d` and `.x`) | 3,130 |
| `opts-omsi-vehicles` | `formats/vehicles/`, `setvars.rs` + `setvars/` (with `data/setvar_families.json`), and `formats/scenery/light.rs` + `light/` (see the cycle below) | 4,314 |
| `opts-omsi-map` | `formats/map/` (including `terrain.rs` and `water.rs`) and `formats/chrono/` | 4,133 |
| `opts-omsi-scenery` | `formats/scenery/` without `light` (`.sco`, `.sli`, crossings), and `formats/trains/` | 3,132 |
| `opts-omsi` (facade) | `lib.rs` and `formats/mod.rs`: re-exports only, plus the `hof`, `wav` and `script_texture` re-exports that stay | 31 |

Sum of the five new crates: 17,999 (+31 facade = 18,030, the measured total before the split).

Every crate is far below 6,000 code lines; the largest is `opts-omsi-vehicles` at 4,314.

### Why these edges and not others

- `scenery` and `vehicles` reference each other: `vehicles/model` uses `scenery::ScoLight` and
  `scenery::light::{parse_light_enh, parse_light_enh_2}`; `scenery/sco.rs` uses
  `vehicles::model::{parse_newanim, ModelAnim}`. The light reader (`light.rs` + tests, 192 lines,
  depending only on `text`) moves into `opts-omsi-vehicles` under its old path
  `formats::scenery::light`; `opts-omsi-scenery` re-exports it. The remaining
  scenery-to-vehicles edge (`sco.rs` to `vehicles::model`) is then one-way.
- `terrain.rs` and `water.rs` use `map::error::MapParseError`. They stay in `opts-omsi-map`: moving
  them to the binary crate would make it depend on the map error type, which in turn needs the map
  crate (a cycle). The binary crate is therefore `o3d` and `x`.
- `chrono` depends only on `map` and `text`, so it joins the map crate.
- `trains` depends on `vehicles::BusManifest` and `text`; it joins `scenery` (above `vehicles`).
- `setvars` depends on `cti` (text); it joins `vehicles` (the vehicle equipment catalogue).

## Dependency edges (no cycles)

```
opts-omsi (facade)  --> opts-omsi-text, -binary, -vehicles, -map, -scenery, opts-wav
opts-omsi-scenery   --> opts-omsi-binary, opts-omsi-vehicles, opts-omsi-text
opts-omsi-map       --> opts-omsi-text, opts-types
opts-omsi-vehicles  --> opts-omsi-text
opts-omsi-binary    --> opts-omsi-text
opts-omsi-text      --> (none)
```

- Each new crate is Layer 1 (shared types, native model, legacy readers), like `opts-omsi`.
- `opts-types` loses the facade edge (`opts-omsi --> opts-types`) and gains `opts-omsi-map`.
  `opts-omsi-vehicles` and `opts-omsi-text` do not use `opts-types`.
- `opts-wav` stays a dependency of the facade only (`formats::wav` is re-exported there).
- Engine crates are unchanged: they reach OMSI only through `opts-core` (facade) and
  `opts_omsi` paths, so `scripts/omsi_uses.py` counts do not change.

## Facade mechanism (no caller changes)

Each new crate has `src/lib.rs` with `pub mod formats;` and a `src/formats/mod.rs` that declares
its own modules and re-exports the sibling-crate modules it refers to, for example
`pub use opts_omsi_text::formats::{cti, constfile, oft, text};` in `opts-omsi-vehicles`. Moved
code keeps its `crate::formats::...` paths, so it does not change. `opts-omsi`'s
`formats/mod.rs` replaces each `pub mod X;` with `pub use opts_omsi_<group>::formats::X;` (module
re-exports), and `setvars` with `pub use opts_omsi_vehicles::setvars;`. Old paths
(`opts_omsi::formats::vehicles::...`, `opts_core::formats::...`, `opts_core::setvars`) keep working.

Data file: `crates/opts-omsi/data/setvar_families.json` moves with `setvars` to
`crates/opts-omsi-vehicles/data/`; `setvars/curated.rs` keeps its `../../data/` include path.

Visibility: a `pub(crate)` item used across a new crate boundary becomes `pub` (no other change).
A moved item referred to by a doc link across crates keeps its path through the re-exports.

## Budgets

Measured after the split (`structure_audit.py check`, code lines in each `src/`). The plan's
estimates are in the table above; the measured figures are the authority and are the budget entries
in `docs/dev/crate-budget.txt`:

| Crate | Measured | Budget entry |
| :--- | ---: | ---: |
| `opts-omsi-text` | 3,307 | 3,307 (new) |
| `opts-omsi-binary` | 3,136 | 3,136 (new) |
| `opts-omsi-vehicles` | 4,323 | 4,323 (new) |
| `opts-omsi-map` | 4,139 | 4,139 (new) |
| `opts-omsi-scenery` | 3,140 | 3,140 (new) |
| `opts-omsi` (facade) | 31 | 31 (lowered from 18,030) |

Sum of the new crates 18,045 (the crates keep their own `lib.rs` and module doorways, so the
total is slightly above the 18,030 before the split). Largest crate: `opts-omsi-vehicles` at 4,323,
under the 6,000 limit. No budget was raised.

Deviations from the plan above: `opts-omsi` drops the dependencies its re-exports no longer use
(`opts-types`, `serde`, `serde_json`, `image`, `thiserror`, `tracing`); `opts-omsi-vehicles` owns
`data/setvar_families.json` (the include path is unchanged inside the crate); `opts-omsi-map` keeps
`terrain.rs` and `water.rs` (they use `MapParseError`, see the cycle note above).

## Crate graph page

For each new crate: a row in the crate table (`Layer 1`, purpose, dependencies, depended by), a
diagram node in Layer 1, the edges listed above, and the `opts-omsi` row updated (dependencies:
the five crates and `opts-wav`).

## Steps

0. This plan (committed first).
1. `opts-omsi-text` (base: everything depends on it). Commit `224367df`.
2. `opts-omsi-binary` (`o3d`, `x`). Commit `a3e4a998`.
3. `opts-omsi-vehicles` (`vehicles`, `setvars`, the `light` reader moved out of `scenery`). Commit `1a8a6991`.
4. `opts-omsi-map` (`map`, `chrono`). Commit `f98234fc`.
5. `opts-omsi-scenery` (`scenery` without `light`, `trains`); the facade keeps only re-exports. Commit `d1b4a151`.

Each step: the workspace compiles; `cargo-seq.sh test --workspace --no-fail-fast` passes;
`scripts/crate_graph.py --check`, `scripts/structure_audit.py check` and
`scripts/omsi_uses.py --check` pass; the commit goes through `seq-run.sh git commit`, which runs
the pre-commit hook.

## Gates

- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `scripts/check_links.py`
  and the other pre-commit checks, through the hook.
- Test counts are recorded per step in the step report.
