# 🚌 OPTS — Open Public Transport Simulator

OPTS is a free, open-source public transport simulator written in **Rust** on the **Bevy** engine. Its first goal is to load, natively and from files you already own, the assets of the bus simulator **OMSI 2** — vehicles, scenery, maps and timetables — and run them in a modern engine.

> 🚧 **Status: early development — specification phase.** There is no playable build. The Cargo workspace compiles (`cargo check --workspace`), the crates are placeholders, and the work done so far is **documentation**: draft specifications of the OMSI file formats, derived by clean-room analysis. See the [roadmap](docs/src/roadmap/README.md) and the [BOARD](docs/dev/BOARD.md) for what is in flight.

## 📖 Documentation first

OPTS follows *Documentation-Driven Development*: the specification is written before the code. The book lives in [`docs/`](docs/src/SUMMARY.md) (mdBook; build with `mdbook build docs`). Good starting points:

| I want to… | Read |
| :--- | :--- |
| understand the goals | [Introduction](docs/src/introduction.md), [Vision](docs/src/vision.md) |
| see the plan | [Roadmap](docs/src/roadmap/README.md), [`ROADMAP.md`](ROADMAP.md) |
| read the format specs | [Formats overview](docs/src/formats/overview.md) |
| know the legal ground rules | [ADR-0002: Clean-Room Policy](docs/src/adr/0002-clean-room-policy.md) |
| contribute | [`CONTRIBUTING.md`](CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) |

All format specifications are currently 🟡 **Draft**: structure is verified against a large installed corpus, but the meaning of many fields still needs behavioural tests and maintainer review.

## ⚖️ Legal position

OPTS **does not distribute** any OMSI 2 file or asset. It only reads files you own. The formats are documented by black-box observation and public documentation; OMSI's executables are never disassembled or decompiled, and community tool source code is never copied. Details and limits: [ADR-0002](docs/src/adr/0002-clean-room-policy.md). That document is not legal advice.

## 🧱 Workspace

| Crate | Role |
| :--- | :--- |
| `opts-core` | file-format parsers and data model (no Bevy dependency) |
| `opts-sim` | simulation logic (no Bevy dependency) |
| `opts-bevy` | Bevy integration: asset loaders, rendering |
| `opts-game` | the game executable |
| `opts-tools` | command-line inspection and conversion tools |
| `opts-utils` | shared helpers |

## 🤝 Contributing and licence

Contributions are welcome under the rules in [`CONTRIBUTING.md`](CONTRIBUTING.md). OPTS is dual-licensed under **MIT OR Apache-2.0** ([`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE)).
