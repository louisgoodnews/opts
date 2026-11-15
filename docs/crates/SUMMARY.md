# Summary

[Introduction](README.md)

---

# 🏗️ Architecture

- [Proposed Architecture](architecture.md)
- [Crate Catalog — Current & Future](catalog.md)
- [Crate Conventions](conventions.md)
- [Workspace Audit & Migration Plan](audit.md)

---

# 📦 Current Crates

- [Foundation]()
  - [opts-types](opts-types.md)
  - [opts-l10n](opts-l10n.md)
  - [opts-logging](opts-logging.md)
- [Model]()
  - [opts-core](opts-core/README.md) (temporary re-export facade)
  - [opts-model](opts-model.md)
  - [opts-omsi](opts-omsi.md)
  - [opts-importer](opts-importer.md)
- [Pipeline]()
  - [opts-catalogue](opts-catalogue/README.md)
- [Simulation]()
  - [opts-sim](opts-sim/README.md)
  - [opts-traffic](opts-traffic.md)
- [Engine (Bevy)]()
  - [opts-bevy](opts-bevy.md)
  - [opts-gui](opts-gui.md)
- [Applications]()
  - [opts-game](opts-game.md)
  - [opts-cli](opts-cli.md)
  - [opts-tools](opts-tools/README.md)
- [Development]()
  - [opts-benchmark](opts-benchmark.md)

---

# 📜 Decisions

- [ADR-0003: Layered, Highly Modular Crate Architecture](adr/0003-layered-crate-architecture.md)
