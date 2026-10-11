# `opts-benchmark`

| Metadata | Value |
| :--- | :--- |
| **Layer** | Development (dev-only, `publish = false`) |
| **Role** | Benchmarks and corpus verification |
| **Kind** | Library (`opts_benchmark`) + binary (`opts-benchmark`) |
| **Status** | 🟡 Scenery harness implemented (M-0054 Phase B); Criterion suite (M-0028) not started |
| **Last Updated** | 2026-10-11 |

## 🎯 Purpose

`opts-benchmark` measures the engine against an installed OMSI corpus. Its first tool is the
**scenery object load verification** of [M-0054](../src/roadmap/phase-5/M-0054.md): every
`Sceneryobjects/**/*.sco` is placed alone on a generated one-tile map, rendered off screen and
compared with the silhouette its own files predict. The design is in
[Scenery Object Load Verification](../src/architecture/scenery-benchmark.md).

The Criterion benches of M-0028 will live in `benches/` and build with `--no-default-features`.

## 🚀 Usage

```sh
# Whole corpus, 3 workers, results under ~/.local/share/opts/benchmark/<run-id>/
opts-benchmark scenery run --omsi-path "<OMSI 2>" --workers 3

# A spread sample, keeping every image, with a throw-away decoder cache
opts-benchmark scenery run --omsi-path "<OMSI 2>" --sample 300 --keep-images all \
    --out /tmp/runs --cache-dir /tmp/cache

# Resume an interrupted run, rewrite summaries, compare two runs
opts-benchmark scenery run --omsi-path "<OMSI 2>" --out /tmp/runs --resume <run-id>
opts-benchmark scenery report --run-dir /tmp/runs/<run-id> --compare /tmp/runs/<older-run-id>
```

The installation is only read. Workers get their own `XDG_CONFIG_HOME` and `XDG_DATA_HOME`
below the run directory; the game's catalogue is opened read-only. The decoder cache
(`$XDG_CACHE_HOME/opts/descramble`) is shared with the game unless `--cache-dir` is given.
To run the tool itself with isolated settings, prefix it with
`XDG_CONFIG_HOME=$(mktemp -d)` and pass `--catalogue ~/.config/opts/catalogue.db`.

## 🧩 Modules (`scenery`)

| Module | Responsibility |
| :--- | :--- |
| `cli` | clap arguments, `execute` |
| `supervisor` | run directory, `run.json`, dynamic dispatch to workers, crash/hang restart, resume |
| `protocol` | `@@opts-benchmark {json}` lines between worker and supervisor |
| `corpus` | `.sco` enumeration, tiers (map tiles scanned), selection (`--filter`, `--list`, `--sample`, `--limit`) |
| `worker` | the windowless Bevy app on `opts-game`'s loader: place, settle, capture |
| `object` | the per-object loop: preflight, place, settle, diagnose, render, analyse, judge, record |
| `preflight` | `.sco` parse, keywords, unsupported tags, context classes, string variables (sets A/B, tree sizes) |
| `expected` | expected triangles (loader's mesh interpretation, independent placement) |
| `framing` | V1 oblique perspective / V2 top-down orthographic, map ↔ Bevy axes |
| `capture` | four cameras (two views × black/white), `Readback` |
| `raster` | masks and a CPU triangle rasteriser |
| `metrics` | two-colour matting, expected masks, per-view metrics |
| `verdict` | thresholds and categories |
| `diag` | tracing capture layer, baseline filter, load-stage categories |
| `lookups` | catalogue-backed path lookups (read-only) |
| `images` | PNGs and repro folders for non-pass objects |
| `report` | `results.jsonl`, `summary.csv`, `categories.json`, `report.md`, `--compare` |
| `script_check` | Phase C extension point (scenarios S0–S5; `S-NOT-RUN` today) |

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-game` (feature `scenery`) | the real map loader (`begin_world_load`, `load_next_tile`, `plan_scene_reload`) |
| `opts-bevy` (feature `scenery`) | mesh parsing, plain twins, decoder cache, the missing-texture sentinel |
| `opts-catalogue` (feature `scenery`) | read-only path lookups, as in the game |
| `bevy` (feature `scenery`) | off-screen rendering and readback |
| `opts-core` | `.sco`/map parsers and the map writers |
| `sha2`, `globset`, `image`, `serde_json`, `clap`, `chrono` | object keys, filters, PNGs, reports, CLI, run ids |

## 🧭 Design Decisions

- One crate for the scenery harness and the future Criterion suite; the renderer sits behind
  the default feature `scenery` (spec, "Crate layout and CLI").
- Workers are separate processes, so a panic or GPU hang costs one restart, not the run.
- The placement transform is computed independently of the game; the mesh interpretation is
  the loader's own (self-consistency, Q-SB-10).

## 🚧 Current State & Known Issues

- Phase C (script effects) reports `S-NOT-RUN`; see Q-SB-11.
- GPU-less CI (software Vulkan) is not verified (Q-SB-01).
- Expected silhouettes ignore back-face culling; see the calibration notes in the spec.

## 🔗 Cross-References

- [Scenery benchmark spec](../src/architecture/scenery-benchmark.md)
- [M-0054](../src/roadmap/phase-5/M-0054.md)
- [ADR-0002 clean-room policy](../src/adr/0002-clean-room-policy.md)
