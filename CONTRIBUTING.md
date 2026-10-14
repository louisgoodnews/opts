# 🤝 Contributing to OPTS

First off, thank you for considering contributing to OPTS! It is people like you who will make an open-source public transport simulator a reality.

This document outlines the philosophy, standards, and processes for contributing to the project. By participating, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📖 The OPTS Way: Documentation-Driven Development

OPTS is not just a coding project; it is a reverse-engineering and documentation project. We follow **Documentation-Driven Development (DDD)**.

**The golden rule:** *Write the specification before the implementation.*

If you want to add a new feature, parse a new OMSI format, or change the architecture, you must first write or update the corresponding documentation in `docs/`. Once the documentation is approved, you can write the code to implement it.

---

## 🚀 Getting Started

### Prerequisites
- **Rust:** Install the latest stable toolchain via [rustup](https://rustup.rs/).
- **Git:** For version control.
- **OSM/GTFS Data (Optional):** Useful for testing simulation logic.

### Setup
1. **Fork the repository** on GitHub.
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/opts.git
   cd opts
   ```
3. **Build the workspace** to ensure everything compiles:
   ```bash
   cargo check
   ```
4. **Run the documentation server** to read the specs:
   ```bash
   cargo install mdbook
   mdbook serve docs
   ```

---

## 🗂️ Workspace Structure

OPTS is organized as a Cargo workspace. Please ensure your contributions go into the correct crate:

| Crate | Layer | Purpose | Dependencies Allowed |
| :--- | :---: | :--- | :--- |
| `opts-logging` | L0 | Structured logging, non-blocking writers, subsystem routing, panic hook. | Zero internal OPTS dependencies |
| `opts-core` | L1 | Pure data structures and parsers for OMSI formats. Zero Bevy, zero file I/O. | `nom`, `binrw`, `serde`, `glam`, `thiserror`, `opts-logging` |
| `opts-sim` | L2 | Engine-agnostic vehicle simulation logic, kinematics, drivetrain, script VM. Zero Bevy. | `opts-core`, `petgraph`, `rayon`, `opts-logging` |
| `opts-traffic` | L2 | AI road traffic, lane graph from OMSI paths, car following, conflict zones. Zero Bevy. | `opts-core`, `opts-logging`, `glam`, `petgraph` |
| `opts-catalogue` | L2 | Scans local OMSI directories into a searchable asset catalogue. Zero Bevy. | `opts-core`, `opts-logging`, `rusqlite` |
| `opts-hardware` | L2 | Privacy-respecting hardware capability detection and preset defaults. Zero Bevy. | `opts-logging` |
| `opts-gui` | L2 | Material Design 3 GUI widget library, design tokens, layout primitives. | `bevy`, `opts-core`, `opts-logging` |
| `opts-bevy` | L2/L3 | Bevy plugin, 3D asset loaders, coordinates, ECS vehicle physics bridge, control server. | `opts-core`, `opts-sim`, `opts-traffic`, `bevy`, `image`, `ddsfile`, `opts-logging` |
| `opts-game` | L3 | The main interactive simulator executable that ties everything together. | `opts-core`, `opts-sim`, `opts-bevy`, `opts-gui`, `opts-catalogue`, `opts-traffic`, `opts-logging`, `bevy` |
| `opts-cli` | L3 | Command-line client for remote controlling a running simulator via JSON-RPC. | `opts-logging`, `clap`, `serde_json` |
| `opts-tools` | L3 | Developer CLI utilities: asset validation (`opts inspect`), metrics, and localization. | `opts-core`, `opts-logging`, `clap`, `anyhow` |
| `opts-benchmark` | L3 | Performance profiling harness and off-screen scenery object verification. | `opts-core`, `opts-bevy`, `opts-sim`, `opts-logging` |

**⚠️ Important Boundary Invariants:**
- `opts-core`, `opts-sim`, `opts-traffic`, `opts-catalogue`, and `opts-hardware` must **never** depend on Bevy, and must not perform synchronous file I/O.
- `opts-logging` must **never** depend on any OPTS internal crate.
- Only `opts-bevy`, `opts-gui`, `opts-game`, and `opts-benchmark` are permitted to depend on Bevy.

---

## 🛠️ How Can I Contribute?

### 🐛 Reporting Bugs
- Check the existing GitHub Issues to see if the bug has already been reported.
- If not, open a new issue using the **Bug Report** template.
- Include: Steps to reproduce, expected behavior, actual behavior, and your OS/Rust version.

### 💡 Suggesting Enhancements
- Open a GitHub Issue using the **Feature Request** template.
- Explain *why* this feature is useful and *how* it aligns with the [Roadmap](ROADMAP.md).

### 📝 Writing Specifications (Docs-First)
- If you discover how an OMSI format works, document it in `docs/src/formats/`.
- Follow the "Two-Phase" process outlined in [ADR-0002](docs/src/adr/0002-clean-room-policy.md).
- Include a **Provenance** section at the bottom of your spec citing exactly how you learned the information.
- Follow the **[Defect Tracking Protocol](docs/src/quality/README.md#defect-tracking-protocol-dual-documentation)** when logging undocumented keywords, scrambled meshes, or parser failures discovered in OMSI assets: document technical details in the format spec's Open Questions table (`Q-NNN`) and record the discovery in the content quality registry.

### 💻 Writing Code
- Ensure there is an approved specification or issue for the code you are writing.
- Write unit tests for your code. For parsers, use self-authored (non-copyrighted) test fixtures.
- Run the local CI suite before submitting or committing:
  ```bash
  ./scripts/local-ci.sh      # Linux / macOS
  .\scripts\local-ci.ps1     # Windows PowerShell
  ```
- **Optional Git pre-commit hook:** You can install the pre-commit hook to automatically verify code formatting and run Clippy checks before each commit:
  ```bash
  ln -s ../../scripts/pre-commit .git/hooks/pre-commit
  ```
  *(The hook is opt-in and is not installed automatically).*

### 🤖 AI Agent Sessions: One Work Station Each

Several agent sessions may work on the repository at the same time. **Each session must have its own work station**: a separate `git worktree` with its own branch.

```sh
scripts/agent-worktree.sh <topic> [base-branch]   # creates ../opts-wt-<topic> on branch agent/<topic>
```

- Never edit or build in the main checkout while another session might be using it, and never check out a branch that another worktree holds.
- Commit only files your session changed. Before committing, check `git status` and `git diff --cached` for hunks that aren't yours.
- When done, hand the branch back (merge or pull request); the maintainer merges it into the main line. Remove the worktree afterwards (`git worktree remove ../opts-wt-<topic>`).
- Build directories are large (the shared `target/` was 116 GB on 2026-10-11). A worktree gets its own `target/` by default; point `CARGO_TARGET_DIR` at a shared directory only when no other session is building, because Cargo serialises builds on one directory.

Background: on 2026-10-11 two sessions shared one working tree, and one of them committed the other's unfinished hunks, which did not compile (`7f24124`). See the [critical review 2026-10-11](docs/src/meta/critical-review-2026-10-11.md) (F-001) and [M-0052](docs/src/roadmap/phase-5/M-0052.md).

---

## ⚖️ Legal & Ethical Guidelines (Crucial)

OPTS is strictly bound by **[ADR-0002: Clean-Room Reverse Engineering Policy](docs/src/adr/0002-clean-room-policy.md)**. 

By contributing to this project, you **affirm** that:
1. You have **not** decompiled, disassembled, or reverse-engineered the OMSI 2 executable (`OMSI.exe`) or any of its proprietary binaries.
2. You have **not** copied code from GPL-licensed community projects (like `Blender-O3D-IO`) into OPTS.
3. You are writing your code based solely on the specifications in `docs/src/formats/` or your own black-box observations.
4. You are **not** distributing any copyrighted OMSI assets (`.o3d`, `.sco`, `.dds`, map files) in your Pull Requests.

**If you are unsure whether your contribution violates this policy, please ask in the GitHub Discussions before opening a Pull Request.** We take this very seriously to protect the project and its contributors.

---

## 🎨 Style Guide

### Rust
- **Formatting:** Run `cargo fmt` on all code.
- **Linting:** Run `cargo clippy --all-targets --all-features -- -D warnings`. Fix all warnings.
- **Error Handling:** Use `thiserror` for library crates (`opts-core`, `opts-sim`) and `anyhow` for binary crates (`opts-game`, `opts-tools`).
- **Documentation:** All public items must have rustdoc comments (`///`). Use `#![warn(missing_docs)]` in `lib.rs`.

### Markdown & Documentation Standards
- **Headings:** Use ATX-style headings (`#`, `##`, `###`).
- **Emojis:** Use them in top-level documents (like the Roadmap and Glossary) to improve visual scanning.
- **Links:** Use relative links for internal docs (e.g., `../glossary.md`) and absolute links for external resources.
- **Character Encoding:** All Markdown files (`*.md`) must strictly be encoded in **UTF-8 without BOM** with `LF` line endings. Never save documentation using legacy single-byte encodings (such as Windows-1252, CP866, or CP1251) which causes mojibake.
- **EditorConfig Recommendation:** Configure your editor to enforce UTF-8:
  ```ini
  [*.md]
  charset = utf-8
  end_of_line = lf
  insert_final_newline = true
  ```
- **Encoding Verification:** You can verify all Markdown files in the repository using:
  ```bash
  find docs/ -name "*.md" -exec sh -c 'file "$1" | grep -q "UTF-8" || echo "Not UTF-8: $1"' _ {} \;
  ```

---

## 📬 Pull Request Process

1. **Create a branch:** Use a descriptive name (e.g., `feat/o3d-parser`, `docs/sco-spec`).
2. **Commit your changes:** Write clear, concise commit messages. Reference the issue number if applicable (e.g., `Fixes #42`).
3. **Push to your fork** and open a Pull Request against the `main` branch.
4. **Fill out the PR template:** 
   - Link to the relevant issue or milestone.
   - **Confirm the Legal Affirmation** (checkbox provided in the template).
   - Describe how you tested the changes.
5. **Wait for review:** A maintainer will review your PR. We may request changes.
6. **Merge:** Once approved and CI passes, your PR will be merged.

---

## 💬 Community

- **GitHub Discussions:** For general questions, ideas, and help.
- **Discord:** [Placeholder for a future Discord invite link]
- **Matrix:** [Placeholder for a future Matrix room]

---

Thank you again for your interest in OPTS. Let's build something amazing together! 🚌
