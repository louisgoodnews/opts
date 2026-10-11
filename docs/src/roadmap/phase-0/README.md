# 🏗️ Phase 0: Foundation & Documentation Infrastructure

*Goal: Establish the project skeleton, workspace, governance and documentation pipeline.*

**Status:** 🟡 Nearly complete

The checklist for this phase lives in the [roadmap](../README.md); this page lists its milestones.

## Milestones

- [M-0000 — Foundation & Documentation Infrastructure](M-0000.md) — 🟡 In Progress (GitHub Pages deployment unverified)

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Establish the project skeleton, workspace, and documentation pipeline.*

*Status 2026-10-04: all items done except the hosted GitHub Pages deployment, which cannot be verified from the working tree. ADR-0002 still carries a `YYYY-MM-DD` placeholder date.*

- [x] **📦 Workspace Setup** *(`cargo check --workspace --offline` passes, verified 2026-10-04)*
  - [x] Initialize Cargo workspace with `opts-core`, `opts-bevy`, `opts-sim`, `opts-tools`, `opts-utils`, and `opts-game`. *(Annotation 2026-10-10: `opts-utils` and other placeholder crates were removed in `07de1b0`; the workspace now has bevy, catalogue, cli, core, game, gui, logging, sim, tools.)*
  - [x] Configure root `Cargo.toml` with shared dependencies and build profiles.
- [ ] **📚 Documentation Pipeline**
  - [x] Set up `mdBook` in the `docs/` directory. *(`docs/book.toml` was a 0-byte file until M-0007; `mdbook build` has not been run locally — the tool is not installed)*
  - [x] Configure `SUMMARY.md` to outline the entire project.
  - [x] Set up GitHub Actions to build and deploy `mdBook` to GitHub Pages on push. *(workflow `deploy-docs.yml` present; it triggers on `main` while the local branch is `master`; first run unverified)*
  - [ ] Configure GitHub Pages and verify the live site updates (needs the hosted repository).
- [x] **⚖️ Project Governance**
  - [x] Add `LICENSE` (MIT/Apache-2.0 dual license).
  - [x] Add `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.
  - [x] Create `docs/src/adr/` for Architecture Decision Records.
  - [x] Write ADR-0001: "Why Bevy and Rust?"
  - [x] Write ADR-0002: "Clean-Room Reverse Engineering Policy" (Crucial for legal safety).
- [x] **📖 Core Documentation**
  - [x] Write `docs/src/introduction.md` and `docs/src/vision.md`.
  - [X] Write `docs/src/glossary.md` (OMSI terms, Bevy terms, transit terminology).
