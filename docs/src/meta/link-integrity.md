# 🔗 Link Integrity Audit Report

- **Date:** 2026-10-06
- **Pass:** Documentation Freeze Pass (Stage 1)
- **Scope:** All Markdown files in repository (`docs/src/`, `docs/crates/`, `docs/dev/`, root)
- **Total Markdown Files Scanned:** 113
- **Total Links Extracted:** 906
- **External Links Checked:** 20 (all syntactically valid)
- **Broken Internal Links Found:** 24 (22 fixed, 2 template/syntax placeholders preserved)

---

## Audit & Resolution Table

| Source File | Line | Link Text | Original Target | Category | Resolution |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `docs/src/contributing/standards.md` | 825 | `CONTRIBUTING.md` | `../../CONTRIBUTING.md` | Trivial / Drift | Fixed to `../../../CONTRIBUTING.md` (root file) |
| `docs/prompts/tasks/Mutation-Test Execution Pass — Task Prompt.md` | 237 | `ADR-0002` | `../../docs/src/adr/0002-clean-room-policy.md` | Drift | Fixed to `../../src/adr/0002-clean-room-policy.md` |
| `docs/crates/architecture.md` | 13 | `Vision, principle 3` | `../vision.html` | Drift / Mismatch | Fixed to `../src/vision.md` |
| `docs/crates/architecture.md` | 105 | `.sco § Encoding` | `../formats/text/sco.html` | Drift / Mismatch | Fixed to `../src/formats/text/sco.md` |
| `docs/crates/architecture.md` | 128 | `format specifications` | `../formats/overview.html` | Drift / Mismatch | Fixed to `../src/formats/overview.md` |
| `docs/crates/architecture.md` | 213 | `ADR-0002` | `../adr/0002-clean-room-policy.html` | Drift / Mismatch | Fixed to `../src/adr/0002-clean-room-policy.md` |
| `docs/crates/architecture.md` | 433 | `ADR-0001: Why Bevy and Rust?` | `../adr/0001-why-bevy-and-rust.html` | Drift / Mismatch | Fixed to `../src/adr/0001-why-bevy-and-rust.md` |
| `docs/crates/catalog.md` | 8 | `roadmap` | `../roadmap/` | Drift | Fixed to `../src/roadmap/README.md` |
| `docs/crates/opts-bevy/README.md` | 55 | `ADR-0001: Why Bevy and Rust?` | `../../adr/0001-why-bevy-and-rust.html` | Drift / Mismatch | Fixed to `../../src/adr/0001-why-bevy-and-rust.md` |
| `docs/crates/opts-bevy/README.md` | 56 | `Phase 3 roadmap` | `../../roadmap/phase-3/` | Drift | Fixed to `../../src/roadmap/phase-3/README.md` |
| `docs/crates/opts-core/README.md` | 58 | `Phase 2 roadmap` | `../../roadmap/phase-2/` | Drift | Fixed to `../../src/roadmap/phase-2/README.md` |
| `docs/crates/opts-sim/README.md` | 58 | `Phase 4 roadmap` | `../../roadmap/phase-4/` | Drift | Fixed to `../../src/roadmap/phase-4/README.md` |
| `docs/crates/opts-tools/README.md` | 54 | `Vision — A Modding Platform` | `../../vision.html` | Drift / Mismatch | Fixed to `../../src/vision.md` |
| `docs/crates/adr/0003-layered-crate-architecture.md` | 23 | `Vision` | `../../vision.html` | Drift / Mismatch | Fixed to `../../src/vision.md` |
| `docs/crates/opts-assets/README.md` | 52 | `Texture formats` | `../../formats/textures.html` | Drift / Mismatch | Fixed to `../../src/formats/textures.md` |
| `docs/crates/opts-game/README.md` | 50 | `Vision — Stage 3 "Driver"` | `../../vision.html` | Drift / Mismatch | Fixed to `../../src/vision.md` |
| `docs/crates/opts-parsers/README.md` | 56 | `Format overview` | `../../formats/overview.html` | Drift / Mismatch | Fixed to `../../src/formats/overview.md` |
| `docs/crates/opts-parsers/README.md` | 57 | `.sco spec` | `../../formats/text/sco.html` | Drift / Mismatch | Fixed to `../../src/formats/text/sco.md` |
| `docs/crates/opts-parsers/README.md` | 57 | `.o3d spec` | `../../formats/binary/o3d.html` | Drift / Mismatch | Fixed to `../../src/formats/binary/o3d.md` |
| `docs/crates/opts-discovery/README.md` | 55 | `Format Inventory` | `../../formats/inventory.html` | Drift / Mismatch | Fixed to `../../src/formats/inventory.md` |
| `docs/crates/opts-benchmark/README.md` | 37 | `ADR-0002` | `../../adr/0002-clean-room-policy.html` | Drift / Mismatch | Fixed to `../../src/adr/0002-clean-room-policy.md` |
| `docs/crates/opts-benchmark/README.md` | 49 | `ADR-0002: Clean-Room Policy` | `../../adr/0002-clean-room-policy.html` | Drift / Mismatch | Fixed to `../../src/adr/0002-clean-room-policy.md` |
| `docs/crates/opts-graphics/README.md` | 53 | `.o3d spec` | `../../formats/binary/o3d.html` | Drift / Mismatch | Fixed to `../../src/formats/binary/o3d.md` |
| `docs/crates/opts-graphics/README.md` | 53 | `Texture property files` | `../../formats/text/texture-cfg.html` | Drift / Mismatch | Fixed to `../../src/formats/text/texture-cfg.md` |
| `docs/src/adr/template.md` | 128 | `Related ADR-XXXX` | `./XXXX-title.md` | Placeholder | Preserved as template example |
| `docs/dev/Documentation Freeze — Task Prompt.md` | 67 | `text` | `path` | Syntax Example | Preserved as prompt markdown example |

---

## External Link Verification

Spot-checked 20 external links across documents (including GitHub repositories, Wikipedia legal case citations, Bevy engine docs, Rust language documentation, LOTUS simulator). All links are well-formed absolute URLs with valid schemes and domains.
