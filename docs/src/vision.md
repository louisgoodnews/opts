# 🔭 Vision

> *Every simulator starts as an impossible idea. This one starts with a decade of buses that deserve a better engine.*

This document describes what OPTS aspires to become. It is deliberately ambitious — it describes a destination that may take years to reach. The [Roadmap](roadmap/README.md) describes the path; the Vision describes the summit.

---

## 🌟 The Elevator Pitch

**OPTS is the open-source home for public transport simulation.**

It is a modern, extensible, community-driven simulator that can load the enormous library of OMSI 2 content, simulate realistic transit operations across a living city, and give modders a platform that will outlast any single game engine.

It is built in Rust, rendered with Bevy, and documented before it is coded.

---

## 🎯 The North Star

Imagine the following experience:

> It's 6:42 AM on a Tuesday in a fictional German city. You're sitting in the driver's seat of a MAN Lion's City bus that a modder built for OMSI 2 in 2015. The engine idles. Rain streaks down the windscreen. Outside, the depot gates are open, and the first passengers of the day are already waiting at the stop across the street.
> 
> You pull out of the depot, following the route your dispatcher assigned. The onboard computer knows every stop, every street name, every timetable. Passengers board, tap their cards, take their seats. An elderly woman asks you a question. A teenager runs to catch the bus just as you're about to pull away.
> 
> Halfway through the route, the city comes alive around you. Cars follow traffic rules. Pedestrians cross at the right moments. Other buses run their own routes on their own schedules. The sun rises over the rooftops.
> 
> At the terminus, you take a short break. In the depot, you can swap to a different vehicle — a tram, or a regional bus, or the same MAN on a different route. The city continues to breathe around you.
> 
> Everything in this world — every bus, every building, every route — came from a community that has been building it for fifteen years. And now it runs on an engine that will still be here in another fifteen.

That is the vision. Everything below is the concrete articulation of it.

---

## 🏛️ Core Values

OPTS is guided by five values. These are not aspirational slogans — they are decision-making tools. When we are uncertain about a direction, we return to these.

### 1. 📖 Documentation First
We write the specification before the implementation. Every format, every system, every architectural decision is documented before it is coded. This is not bureaucracy — it is how we make the project legible to contributors, honest about its limitations, and legally defensible.

See [ADR-0002](adr/0002-clean-room-policy.md).

### 2. ⚖️ Clean and Ethical
We do not distribute copyrighted assets. We do not decompile proprietary binaries. We respect the work of modders, both by crediting it and by making it portable. We comply with licenses, and we ask questions when we are uncertain. See [ADR-0002](adr/0002-clean-room-policy.md).

### 3. 🧩 Extensible by Design
The simulator is a platform, not a product. Every subsystem — vehicles, maps, routing, physics — should be replaceable or extendable by the community. Modding is not an afterthought; it is a first-class design concern.

### 4. 🏎️ Performant and Modern
OPTS targets the hardware of today and tomorrow. Rust and Bevy give us memory safety, parallelism, and a modern rendering pipeline. We will not sacrifice performance for convenience.

### 5. 🌍 Open and Welcoming
OPTS is open source under MIT/Apache-2.0. It is open to contributors of all backgrounds, skill levels, and interests. The project's success depends on a healthy, patient, kind community. See [CODE_OF_CONDUCT](../../CODE_OF_CONDUCT.md).

---

## 🧱 What OPTS Will Be

These are the pillars of the simulator. Each is a major undertaking in its own right.

### 🚌 A Vehicle Simulator
- Multiple vehicle classes: buses, trams, trolleybuses, and more.
- Realistic physics: engine, transmission, suspension, brakes, steering.
- Interactive cabins: doors, indicators, ticket machines, onboard computers.
- Support for OMSI 2 vehicle assets, including scripts for complex systems.

### 🗺️ A World Simulator
- Cities built from OpenStreetMap data and OMSI scenery assets.
- Day-night cycles, weather, and seasonal variation.
- Traffic AI, pedestrian AI, and other ambient systems.
- Dynamic map streaming to handle cities of realistic scale.

### 🚏 A Transit Simulator
- Routes, lines, stops, timetables, and service patterns.
- Vehicles dispatched from depots on schedules.
- Passenger demand, boarding, alighting, and transfers.
- Support for GTFS and GTFS-RT for real-world timetable data.

### 🧑‍🤝‍🧑 A Passenger Simulator
- Passenger agents with destinations and waiting behavior.
- Realistic boarding and alighting times based on crowd density.
- Transfers between routes at interchanges.
- Crowding, standing passengers, and capacity limits.

### 🎮 A Game
- Single-player career mode with progression.
- Scenario editor for community-created challenges.
- Multiplayer (long-term) with dispatcher/driver roles.
- Free-play sandbox mode for creative exploration.

### 🛠️ A Modding Platform
- Open, documented file formats for all OPTS-specific assets.
- Import pipelines for OMSI 2 assets, GTFS data, and OSM data.
- A scripting layer for vehicles and scenarios.
- A tooling suite (`opts-tools`) for inspecting, converting, and validating assets.

---

## 🚫 What OPTS Will Not Be

Boundaries are as important as goals. These are things OPTS deliberately will *not* attempt, either for legal, ethical, or practical reasons.

- **A distribution platform for OMSI 2 assets.** We do not ship copyrighted content. Users bring their own assets.
- **A decompilation project.** We reverse-engineer formats only through black-box observation and public knowledge. See [ADR-0002](adr/0002-clean-room-policy.md).
- **A competitor to OMSI 2 or LOTUS.** OPTS is not trying to "win" against existing simulators. It is trying to preserve and extend the community's work.
- **A commercial product.** OPTS is free, open source, and will remain so.
- **A hyper-realistic physics engine.** We aim for realistic and satisfying, not for professional-grade engineering simulation.
- **A general-purpose game engine.** OPTS is a simulator. It is not a platform for other genres.

---

## 🛣️ The Journey in Stages

The vision unfolds in recognizable stages. These are not dates — they are checkpoints. Each stage marks a meaningful leap in capability.

### Stage 1 — Foundation
*The project exists and is buildable.*
Documentation infrastructure is set up. ADRs are in place. The workspace compiles. Format specifications are being written.

### Stage 2 — Reader
*OPTS can read OMSI 2 files and render them in Bevy.*
A single `.sco` scenery object loads and displays. Then a bus. Then a full map tile. `opts-tools` can inspect and convert assets from the command line.

### Stage 3 — Driver
*OPTS can be played.*
A bus drives through a small city. Physics work. Doors work. Passengers board. The player can complete a route from start to finish.

### Stage 4 — Simulator
*OPTS simulates a transit network.*
Multiple routes, multiple vehicles, and a working timetable. Passengers transfer. Delays propagate. The city runs on a schedule.

### Stage 5 — Platform
*OPTS is a community.*
Modders publish vehicles, maps, and scenarios. A scripting layer lets them extend the simulation. A stable public release is available for Windows, Linux, and macOS.

### Stage 6 — Standard
*OPTS is the modern home for public transport simulation.*
The OMSI 2 community has migrated. New content is being created natively for OPTS. Other transit datasets — GTFS, OSM, real-world timetables — flow into the simulator. The project has outlived the original game it was built to honor.

---

## 💭 A Note on Ambition

This vision is enormous. It will take years. It may never fully be realized. That is okay.

OPTS is a project that will grow one specification, one parser, one Bevy system, and one contributor at a time. Every `.sco` file that loads correctly is a small victory. Every pull request that adds a crumb of capability is progress.

The goal is not to reach the summit on a schedule. The goal is to keep climbing.

---

## 🔗 Related Reading

- [Introduction](introduction.md) — What OPTS is, today.
- [Roadmap](roadmap/README.md) — The concrete path from here to there.
- [Glossary](glossary.md) — Shared vocabulary for the project.
- [ADR-0001: Why Bevy and Rust?](adr/0001-why-bevy-and-rust.md) — The technical foundation of the vision.
- [ADR-0002: Clean-Room Policy](adr/0002-clean-room-policy.md) — The ethical foundation of the vision.

---

*The engine is old. The buses are not. Let's give them a road that lasts.* 🚌
