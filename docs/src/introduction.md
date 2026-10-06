# 👋 Introduction

Welcome to **OPTS** — the **Open Public Transport Simulator**.

OPTS is a free, open-source public transport simulator written in **Rust** and built on the **Bevy** game engine. Its goal is to let you build, drive, and manage buses, trams, and other vehicles in detailed, living cities — starting with the ability to natively load assets created for the classic bus simulator **OMSI 2**.

> 🚧 **Project Status: Early Development.** 
> OPTS is in its foundational phase. We are currently writing specifications, designing the architecture, and preparing the codebase. There is no playable build yet. If you want to follow along or help build it, you're in exactly the right place.

---

## 🎯 What is OPTS?

OPTS is a simulator. Specifically, it aims to simulate:

- 🚌 **Vehicles** — Buses, trams, and eventually other modes of public transport, with realistic physics and control systems.
- 🗺️ **Routes** — Lines, stops, timetables, and the networks that connect them.
- 🧑‍🤝‍🧑 **Passengers** — People who spawn, wait, board, alight, and transfer based on schedules and demand.
- 🌆 **Cities** — Built from OpenStreetMap data and existing OMSI scenery assets.
- 🕐 **Time** — A day-night cycle, service patterns, and the ebb and flow of a real transit system.

The long-term vision is a simulator that is **moddable**, **open**, and **extensible** — a platform where the community can build maps, vehicles, and scenarios without being locked into a closed ecosystem.

---

## 🚌 Why OMSI 2 Assets?

[OMSI 2](https://en.wikipedia.org/wiki/OMSI_2) is a beloved bus simulator released in 2011. Over more than a decade, its community has produced an enormous library of high-quality buses, maps, and scenery objects. That content represents thousands of hours of work by passionate modders.

Unfortunately, OMSI 2 runs on an aging, closed-source engine that is difficult to extend and no longer actively developed. Much of that community content risks becoming stranded on a platform that may not run on modern systems.

OPTS aims to change that by **natively reading OMSI 2 asset formats** (`.o3d`, `.sco`, `.sli`, `.bus`, `.hof`, and more) and rendering them in a modern engine. Our goal is not to replace OMSI 2 or to distribute its assets, but to give the community's work a new home where it can thrive for years to come.

> ⚖️ **A Note on Legality.** 
> OPTS does **not** distribute any OMSI 2 game files or copyrighted assets. We only write software that reads files you already own. Our reverse-engineering process follows a strict clean-room policy. See [ADR-0002](adr/0002-clean-room-policy.md) for the full details.

---

## 🧭 Who Is This Documentation For?

This book is organized to serve four different audiences. Find yourself below and jump to the section that matters most to you.

### 🎮 Players & Transit Enthusiasts
You want to drive a bus, run a route, or explore a city. 
→ Start with the **User Guide *(planned)*** (coming soon).

### 🛠️ Modders & Asset Creators
You make buses, maps, or scenery objects and want to bring them into OPTS. 
→ Start with the **Modding Guide *(planned)*** and the **[OMSI Format Specs](formats/overview.md)**.

### 💻 Rust & Bevy Developers
You want to contribute code, understand the architecture, or extend the engine. 
→ Start with the **Architecture *(planned)*** section and the **Contributing Guide *(planned)***.

### 🧑‍🔬 Simulation & Transport Researchers
You're interested in the simulation model, routing algorithms, or transit data standards like GTFS. 
→ Start with the **Simulation *(planned)*** section.

---

## 📖 How This Documentation Works

OPTS follows a philosophy called **Documentation-Driven Development (DDD)**. This means we write the *specification* before we write the *code*.

Every OMSI file format we support, every simulation system we build, and every architectural decision we make is documented here first. The documentation **is** the source of truth. The code exists to implement what is described in these pages.

This has three important consequences:

1. **The specs are readable.** You don't need to read Rust code to understand how OPTS works. The documentation stands on its own.
2. **The specs are citable.** Every claim in a format spec has a **Provenance** section explaining how we learned it. This is part of our [clean-room policy](adr/0002-clean-room-policy.md).
3. **The specs are living.** As we learn more about OMSI formats or refine our simulation models, the documentation is updated before the code is.

---

## 🗺️ Where to Go Next

Here are some good starting points, depending on what you want to do:

| I want to... | Go to... |
| :--- | :--- |
| Understand the long-term goals of the project | [Vision](vision.md) and [Roadmap](roadmap/README.md) |
| Learn the terminology used throughout the docs | [Glossary](glossary.md) |
| Understand *why* we chose Rust and Bevy | [ADR-0001](adr/0001-why-bevy-and-rust.md) |
| Understand our legal and ethical boundaries | [ADR-0002](adr/0002-clean-room-policy.md) |
| See how OMSI file formats are structured | [OMSI Formats](formats/overview.md) |
| Contribute code or documentation | Contributing *(planned)* |

---

## 🤝 Join the Project

OPTS is being built in the open, and we welcome contributors of all skill levels — whether you're a Rust expert, a transit nerd, a documentation enthusiast, or just someone who loves buses.

If you'd like to help, please read the **Contributing Guide *(planned)*** and the **[Code of Conduct](../../CODE_OF_CONDUCT.md)**.

The project is young, but the vision is big. If you've ever wanted to help build the spiritual successor to OMSI 2, now is the time.

---

*Let's build something worth riding.* 🚌
