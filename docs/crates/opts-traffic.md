# `opts-traffic` — AI Road Traffic

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-traffic` |
| **Layer** | L4 · Simulation Core |
| **Kind** | Library (`lib.rs`), plus the `lane_graph_audit` example |
| **Status** | 🟡 Foundation (nothing wired into the game yet) |
| **Last Updated** | 2026-10-11 |
| **Plan** | [AI Road Traffic — Research and Integration Plan](../src/architecture/ai-traffic.md), [M-0049](../src/roadmap/phase-4/M-0049.md) |

---

## 🎯 Purpose

Pure Rust, deterministic AI traffic: turns the `[path]` lanes of OMSI splines and junction objects into a directed lane graph, moves agents along it with the Intelligent Driver Model and reports the pedal and indicator values an OMSI AI vehicle script expects. Like `opts-sim` it has **no Bevy dependency and does no I/O**; the host supplies parsed data and the clock. Dependencies: `opts-core`, `glam`, `thiserror`, `tracing`.

## 📤 Public API Surface

- **`geometry::PathGeometry`** — straight/arc in OMSI conventions (compass heading, positive radius = clockwise, radius `-999.99` = straight): `point_at`, `heading_at`, `end`, `reversed`, `offset` (lateral lane offset), `to_world` (object-local → world).
- **`graph::{LaneSegment, LaneGraph, ConnectParams, LaneId, LaneKind, Direction}`** — `LaneGraph::build(&segments, &params)` expands directions (`Reverse`, `Both`) and joins lane ends to starts of the same kind within 0.75 m / 15° (default); `successors`, `lane`, `lane_ids`, `shortest_route` (Dijkstra by length).
- **`source::{PathSource, segments_from_tile, tile_origin}`** — converts a parsed `MapTile` (splines with their `.sli` paths, objects with their `.sco` paths) into world-space `LaneSegment`s; the host implements `PathSource` over its caches. Splines/paths with non-positive length are skipped. `[rule]` records are applied per lane: `speedlimit` (km/h → `speed_limit` in m/s), `trafficdensity`, `priority`, vehicle flags (`LaneRules`).
- **`idm::IdmParams`** — Intelligent Driver Model (`acceleration(v, speed_cap, ahead)`); presets `default()` (car, 50 km/h) and `heavy()`. Literature defaults, not calibrated to OMSI.
- **`agent::{TrafficWorld, Agent, AgentId, DriverCommand, Blinker, TrafficEvent}`** — `spawn` (free-room check), `step(dt)` (leaders on the same lane and across the lane border, ballistic update, lane transitions with seeded branch choice, dead-end removal), `command(id)` (`throttle`/`brake` from acceleration, indicators from the next lane's turn), `Agent::pose`.
- **`graph::Elevation`**, **`Lane::z_at`**, **`Lane::key`**, **`LaneGraph::{lane_by_key, random_spot}`** — height profile from the gradients (`rise = length·(g0+g1)/2`), stable lane identity across rebuilds, random placement in a distance band; **`TrafficWorld::replace_graph`** swaps in a rebuilt graph and keeps agents on lanes with the same key.
- **`conflict::Conflict`**, **`LaneGraph::{conflicts, predecessors}`** — crossing and merging zones per lane, computed on first use; the agent step makes vehicles wait in front of a zone when another arrives earlier (`TrafficWorld` keeps a clock and a deadlock breaker).
- **`density::{DensityCurve, weighted_pick}`**, **`rng::Rng`** (SplitMix64) — spawn weights and day curves; all randomness is seeded.

Parsers for the input files live in `opts-core`: `opts_core::formats::map::{AiLists, parse_unsched_vehgroups, parse_unsched_trafficdens}`.

## 🧪 Verification

`cargo test -p opts-traffic` (31 unit tests + a doctest); `OMSI_PATH=… cargo test -p opts-core --test ai_lists_corpus`; `cargo run --release -p opts-traffic --example lane_graph_audit -- "<OMSI>/maps/<Map>"` prints lane counts, connectivity and tolerance sweeps for a real map.

## 🧭 Design Decisions

- Agents are processed in lane / position / id order and use one seeded generator, so runs are reproducible.
- Kinematic and script-driven vehicles share the same agent state; see the plan, §5.
- Not implemented yet: signals, right of way by rule (conflicts are resolved by arrival time), lane changes, scheduled routes (plan §6–§8).
