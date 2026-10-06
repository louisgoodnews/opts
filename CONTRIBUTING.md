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

| Crate | Purpose | Dependencies Allowed |
| :--- | :--- | :--- |
| `opts-core` | Pure data structures and parsers for OMSI formats. | `nom`, `binrw`, `serde`, `glam`, `thiserror` |
| `opts-bevy` | Bevy plugin, asset loaders, and rendering integration. | `opts-core`, `bevy`, `image` |
| `opts-sim` | Engine-agnostic simulation logic (physics, passengers). | `opts-core`, `petgraph`, `rayon` |
| `opts-game` | The main executable that ties everything together. | `opts-bevy`, `opts-sim`, `bevy` |
| `opts-tools` | CLI tools for inspecting and converting OMSI assets. | `opts-core`, `anyhow` |
| `opts-utils` | Shared utilities. Keep this lean. | Minimal |

**⚠️ Important:** `opts-core` must **never** depend on Bevy. It must remain a pure, testable Rust library that could theoretically run in a headless server.

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

### 💻 Writing Code
- Ensure there is an approved specification or issue for the code you are writing.
- Write unit tests for your code. For parsers, use self-authored (non-copyrighted) test fixtures.
- Run `cargo fmt` and `cargo clippy` before committing.

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

### Markdown
- **Headings:** Use ATX-style headings (`#`, `##`, `###`).
- **Emojis:** Use them in top-level documents (like the Roadmap and Glossary) to improve visual scanning.
- **Links:** Use relative links for internal docs (e.g., `../glossary.md`) and absolute links for external resources.

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
