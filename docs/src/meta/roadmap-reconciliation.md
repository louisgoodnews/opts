# 🗺️ Roadmap & Board Reconciliation Report

- **Date:** 2026-10-06
- **Pass:** Documentation Freeze Pass (Stage 6)
- **Sources Reconciled:**
  - Project Root [`ROADMAP.md`](../../ROADMAP.md)
  - Canonical Roadmap [`docs/src/roadmap/README.md`](../roadmap/README.md)
  - Phase READMEs (`docs/src/roadmap/phase-*/README.md`)
  - Milestone Files (`M-0000.md` through `M-0012.md`)
  - Development Board [`docs/dev/BOARD.md`](../../dev/BOARD.md)

---

## Milestone Status Alignment

| Milestone ID | Title | `phase-1/README.md` Before | `M-*.md` Before | Aligned Status | Notes / Rationale |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **M-0000** | Foundation & Documentation | 🟡 In Progress | 🟡 In Progress | **🟡 In Progress** | Core skeleton done; hosted GitHub Pages unverified. |
| **M-0001** | Vehicle Text Formats | 🟢 drafted | 🟢 Complete (drafts) | **🟡 Drafted (review pending)** | Conservative rule: specs exist as 🟡 Draft; review pending. |
| **M-0002** | Binary Mesh Formats | 🟢 complete | 🟢 Complete | **🟢 Complete** | In-engine mutation pass complete; `.o3d` and `.x` 🟢 Specified. |
| **M-0003** | World Format Specifications | 🟡 tutorial blocked | 🟡 In Progress | **🟡 In Progress** | Blocked on core parser implementation; mutation kit pending. |
| **M-0004** | Transit Data Specifications | 🟡 GTFS notes open | 🟡 In Progress | **🟡 In Progress** | `.hof` drafted; GTFS notes open. |
| **M-0005** | Script Format Specifications | 🟢 drafted | 🟢 Complete (drafts) | **🟡 Drafted (review pending)** | Conservative rule: specs exist as 🟡 Draft; review pending. |
| **M-0006** | Community Wiki Cross-Check | 🟢 complete | 🟢 Complete | **🟢 Complete** | 31 wiki pages reconciled across 19 specs. |
| **M-0007** | Documentation Drift Repair | 🟢 complete | 🟢 Complete | **🟢 Complete** | Broken links fixed; phase layout unified. |
| **M-0008** | Script Reference | 🟢 complete | 🟢 Complete | **🟢 Complete** | Predefined variables and callbacks documented. |
| **M-0009** | Repaint System | 🟢 complete | 🟢 Complete | **🟢 Complete** | `.cti` / `[CTC]` chain documented. |
| **M-0010** | Engine & Environment Configs | 🟢 complete | 🟢 Complete | **🟢 Complete** | `envir.cfg`, weather, inputs documented. |
| **M-0011** | Passenger Content | 🟢 complete | 🟢 Complete | **🟢 Complete** | Ticket packs, money, fonts documented. |
| **M-0012** | Chrono and Map Variants | 🟢 complete | 🟢 Complete | **🟢 Complete** | Chrono overlays and variants documented. |

---

## Development Board Actions

- Reconciled Backlog item for `SUMMARY.md` draft chapters (marked done following Stage 5 reconciliation).
- Reconciled Deferred Cleanup item for 0-byte `docs/crates/book.toml` (confirmed populated in M-0007).
- Updated board header date to `2026-10-06`.
