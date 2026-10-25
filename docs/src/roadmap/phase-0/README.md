# 🏗️ Phase 0: Foundation & Documentation Infrastructure

*Goal: Establish the project skeleton, workspace, governance and documentation pipeline.*

**Status:** 🟢 Complete (status sync 2026-10-25: book live at https://louisgoodnews.github.io/opts/)

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0000 — Foundation & Documentation Infrastructure](M-0000.md) — 🟢 Complete (book live on GitHub Pages, status sync 2026-10-25)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Establish the project skeleton, workspace, and documentation pipeline.*

*Status 2026-10-25: all items done. The repository is hosted (github.com/louisgoodnews/opts) and the book is live at https://louisgoodnews.github.io/opts/ (`deploy-docs.yml` green since 2026-10-13). ADR-0002 is dated 2026-10-04.*

- [x] **📦 Workspace Setup** *(`cargo check --workspace --offline` passes, verified 2026-10-04)*
  - [x] Initialize Cargo workspace with `opts-core`, `opts-bevy`, `opts-sim`, `opts-tools`, `opts-utils`, and `opts-game`. *(Annotation 2026-10-10: `opts-utils` and other placeholder crates were removed in `07de1b0`; the workspace now has bevy, catalogue, cli, core, game, gui, logging, sim, tools.) (Status sync 2026-10-25: 17 members — benchmark, bevy, catalogue, cli, core (facade), game, gui, hardware, importer, l10n, logging, model, omsi, sim, tools, traffic, types.)*
  - [x] Configure root `Cargo.toml` with shared dependencies and build profiles.
- [ ] **📚 Documentation Pipeline**
  - [x] Set up `mdBook` in the `docs/` directory. *(`docs/book.toml` was a 0-byte file until M-0007; `mdbook build docs` succeeds locally with mdbook 0.5.4, verified 2026-10-25, warnings only)*
  - [x] Configure `SUMMARY.md` to outline the entire project.
  - [x] Set up GitHub Actions to build and deploy `mdBook` to GitHub Pages on push. *(workflow `deploy-docs.yml` present; triggers on `main` and `master`; green on GitHub since 2026-10-13)*
  - [x] Configure GitHub Pages and verify the live site updates (live at https://louisgoodnews.github.io/opts/, verified 2026-10-25).
- [x] **⚖️ Project Governance**
  - [x] Add `LICENSE` (MIT/Apache-2.0 dual license).
  - [x] Add `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.
  - [x] Create `docs/src/adr/` for Architecture Decision Records.
  - [x] Write ADR-0001: "Why Bevy and Rust?"
  - [x] Write ADR-0002: "Clean-Room Reverse Engineering Policy" (Crucial for legal safety).
- [x] **📖 Core Documentation**
  - [x] Write `docs/src/introduction.md` and `docs/src/vision.md`.
  - [X] Write `docs/src/glossary.md` (OMSI terms, Bevy terms, transit terminology).
- [x] **📋 Milestone file** — [M-0000](M-0000.md) (🟢 complete 2026-10-25: Pages live)
