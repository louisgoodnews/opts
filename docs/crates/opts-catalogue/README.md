# `opts-catalogue`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L3 · Pipeline |
| **Role** | member |
| **Kind** | lib |
| **Status** | 🟢 Implemented (schema v1) |
| **Last Updated** | 2026-10-09 |

## 🎯 Purpose

SQLite cache of an OMSI 2 installation: every directory and file (`rel_path` indexed `COLLATE NOCASE` for O(1) path resolution), discovered content entities (vehicles, maps, scenery objects, splines, humans, …), their display metadata and references. Replaces the retired `opts-discovery` skeleton. Design: [catalogue-schema.md](../../src/architecture/catalogue-schema.md), [playability-rules.md](../../src/architecture/playability-rules.md).

## 📤 Public API

- `discover(db_path, root, progress) -> Summary` — parallel walk, entity detection, one-transaction rewrite.
- `Catalogue::open`, `staleness(root, max_age)`, `find_file`, `find_dir`, `maps`, `playable_vehicles`, `vehicle_marker`, `totals`, `last_scan`, `category_counts`.
- Modules: `scan` (filesystem walk), `extract` (marker rules, metadata, playability R1–R5), `schema` (DDL, version handshake).

## 📦 Dependencies

`opts-core` (parsers), `rusqlite` (bundled), `rayon`, `thiserror`, `tracing`.

## 🧪 Verification

Unit tests on a synthetic installation; `examples/scan_omsi.rs` scans a real one (≈ 4.5 s for 622k files in release).
