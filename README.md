# 🚌 OPTS — Open Public Transport Simulator

OPTS is a free, open-source public transport simulator written in **Rust** on the **Bevy** engine. Its first goal is to load, natively and from files you already own, the assets of the bus simulator **OMSI 2** — vehicles, scenery, maps and timetables — and run them in a modern engine.

> 🚧 **Status: early development — a runnable prototype, not a release.** `opts-game` starts, finds your OMSI 2 installation (or asks for it), lists your maps and vehicles, loads a map with terrain, roads, scenery, sky and parked vehicles, and lets you drive a vehicle whose original OMSI scripts run: animations, lights, doors, indicators and the cockpit's clickable controls respond. Passengers, timetables, AI traffic and sound are not implemented yet, and some OMSI meshes whose vertex data is scrambled still cannot be shown. Format specifications remain drafts. See the [roadmap](ROADMAP.md) and the [BOARD](docs/dev/BOARD.md) for what is in flight.

## ✅ What works today

| Area | State |
| :--- | :--- |
| **Parsers** (`opts-core`) | `.o3d`, `.x`, `.sco`, `.sli`, map tiles, terrain, water, `.bus`, `model.cfg`, `paths.cfg`, `.org`, money, humans, `.cti`, `keyboard.cfg`, constfiles and varlists |
| **World** | tile streaming, terrain with layered ground shader, roads and spline attachments, sky from the map's environment, text textures on signs and stops, parked vehicles |
| **Vehicles** | kinematic/dynamic driving, wheels, cockpit/chase/passenger/free-roam cameras, driver character |
| **Scripts** (`opts-sim`) | OMSI `.osc` script VM (compiles all 6,062 scripts of the reference corpus), driving mesh animation, lights, light maps, OMSI key bindings and cockpit hot-spots |
| **Tooling** | `opts-cli` remote control of a running game, hot reload of changed assets, F1 bug reporting, `opts-tools inspect` |
| **Not yet** | passengers and routing, timetables, AI traffic, spatial audio, night maps, in-game settings, packaged releases |

## 🚀 Running it

OPTS never ships OMSI content: you need your own OMSI 2 installation.

```sh
cargo run -p opts-game                          # asks for the OMSI folder on first start
cargo run -p opts-game -- --omsi-path "/path/to/OMSI 2"
```

The installation is also found through `$OMSI_PATH` and the default Steam folders. Developer switches are listed in the [developer switches reference](docs/src/contributing/developer-switches.md). The Rust toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml); the engine is **Bevy 0.19** (0.20 is planned).

## 📖 Documentation first

OPTS follows *Documentation-Driven Development*: the specification is written before the code. The book lives in [`docs/`](docs/src/SUMMARY.md) (mdBook; build with `mdbook build docs`). Good starting points:

| I want to… | Read |
| :--- | :--- |
| understand the goals | [Introduction](docs/src/introduction.md), [Vision](docs/src/vision.md) |
| see the plan | [`ROADMAP.md`](ROADMAP.md), [book copy](docs/src/roadmap/README.md) |
| read the format specs | [Formats overview](docs/src/formats/overview.md) |
| know the legal ground rules | [ADR-0002: Clean-Room Policy](docs/src/adr/0002-clean-room-policy.md) |
| contribute | [`CONTRIBUTING.md`](CONTRIBUTING.md), [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) |

Format specifications cover every OMSI 2 file type found in the reference corpus. The two mesh formats (`.o3d`, `.x`) are 🟢 verified by mutation tests in OMSI; the others are 🟡 drafts whose structure is verified against a large installed corpus while the meaning of some fields still needs behavioural tests and maintainer review.

## ⚖️ Legal position

OPTS **does not distribute** any OMSI 2 file or asset. It only reads files you own. The formats are documented by black-box observation and public documentation; OMSI's executables are never disassembled or decompiled, and community tool source code is never copied. Details and limits: [ADR-0002](docs/src/adr/0002-clean-room-policy.md). That document is not legal advice.

## 🧱 Workspace

| Crate | Role |
| :--- | :--- |
| `opts-core` | file-format parsers and data model (no Bevy dependency) |
| `opts-sim` | vehicle physics and the OMSI script VM (no Bevy dependency) |
| `opts-bevy` | Bevy integration: asset loaders, world and vehicle rendering, script bridge, camera, control protocol server, hot reload |
| `opts-gui` | Material Design 3 widget library for Bevy UI |
| `opts-game` | the game executable: menus, map and vehicle selection, in-game world |
| `opts-catalogue` | scans an installation into a map/vehicle catalogue |
| `opts-cli` | command-line client that remote-controls a running game |
| `opts-tools` | file inspection tools |
| `opts-logging` | logging setup and log files |

## 🤝 Contributing and licence

Contributions are welcome under the rules in [`CONTRIBUTING.md`](CONTRIBUTING.md). OPTS is dual-licensed under **MIT OR Apache-2.0** ([`LICENSE-MIT`](LICENSE-MIT), [`LICENSE-APACHE`](LICENSE-APACHE)).
