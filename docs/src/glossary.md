# 📖 Glossary

This glossary defines the shared vocabulary used throughout the OPTS project. Because OPTS bridges the worlds of OMSI 2 modding, Bevy game development, and public transport simulation, this document is the source of truth for terminology.

If you encounter a term that is not defined here, please open a pull request to add it.

---

## 🏗️ Project Terminology

| Term | Definition | Notes |
| :--- | :--- | :--- |
| **OPTS** | **Open Public Transport Simulator.** The name of this project. An open-source simulator built in Rust that loads OMSI 2 assets. | See [Introduction](introduction.md). |
| **ADR** | **Architecture Decision Record.** A document that captures a significant architectural decision, its context, and its consequences. | See `docs/src/adr/`. |
| **Clean-Room Reverse Engineering** | The practice of documenting a proprietary format based on observed behavior and public knowledge, without ever looking at the original proprietary source code, to avoid copyright infringement. | See ADR-0002. |
| **DDD** | **Documentation-Driven Development.** The philosophy of this project. Writing specifications and documentation *before* writing implementation code. | See [Vision](vision.md). |
| **Milestone (M-XXXX)** | A unit of work that groups related tasks towards a specific goal. Named with `M-` followed by a four-digit number. | See `docs/roadmap/`. |
| **Phase** | A high-level stage of the project, containing multiple milestones. | See [Roadmap](roadmap/README.md). |

---

## 🚌 OMSI 2 & Asset Formats

| Term | Definition | Notes |
| :--- | :--- | :--- |
| **OMSI** | **Omnibus Simulator.** A popular, older bus simulation game released in 2011. OPTS aims to load its assets. | The original game is proprietary; OPTS does not distribute its assets. |
| **`.o3d`** | The proprietary binary 3D mesh format used by OMSI for models. | See [o3d.md](formats/binary/o3d.md). |
| **`.sco`** | **Scenery Object.** A plain-text configuration file that defines how a 3D model (`.o3d`) is used in the world (textures, collision, placement rules). | See [sco.md](formats/text/sco.md). |
| **`.sli`** | **Spline.** A plain-text configuration file that defines a 2D profile which is extruded along a path to create roads, tracks, and fences. | See [sli.md](formats/text/sli.md). |
| **`.bus` / `.cfg`** | Configuration files that define a vehicle's model hierarchy, parts, and scripts. | See [bus.md](formats/text/vehicles/bus.md). |
| **`.hof`** | A file containing bus stop names, route data, and IBIS codes for a specific map. | See [hof.md](formats/text/vehicles/hof.md). |
| **`.dds`** | **DirectDraw Surface.** The primary texture format used by OMSI. Supports compression and alpha channels. | See [textures.md](formats/textures.md). |
| **`.osc`** | OMSI Script. A proprietary scripting language used to control vehicle logic (lights, doors, etc.). | Parsing this is a future goal. |
| **IBIS** | **Integrated Board Information System.** The system used in buses to manage routes, stops, and passenger announcements. | Data is stored in `.hof` files. |
| **Spline** | A mathematical curve used to define paths. In OMSI, roads are splines with a 2D cross-section profile. | Not to be confused with Bevy's built-in `Spline` types. |
| **Scenery Object** | Static objects placed in the world, such as buildings, trees, and bus stops. | Defined by `.sco` files. |

---

## 🎨 Bevy & Rust Ecosystem

| Term | Definition | Notes |
| :--- | :--- | :--- |
| **Bevy** | A modern, open-source game engine written in Rust, built on an ECS architecture. | The rendering engine for OPTS. See ADR-0001. |
| **ECS** | **Entity Component System.** An architectural pattern where data (Components) is associated with IDs (Entities) and logic is run by Systems. | The core paradigm of Bevy. |
| **Entity** | A unique identifier in the Bevy ECS. It has no data itself; it is just an ID. | Think of it as a row in a database. |
| **Component** | Plain data attached to an Entity (e.g., `Transform`, `Mesh`, `Texture`). | Think of it as a column in a database. |
| **System** | A function that runs logic on Entities that have specific Components. | e.g., `move_vehicles_system`. |
| **Plugin** | A modular piece of functionality that can be added to a Bevy `App`. | `opts-bevy` is a Plugin. |
| **AssetLoader** | A Bevy trait for defining how to load custom file formats (like `.o3d`) into the engine. | See `opts-bevy`. |
| **Bundle** | A group of Components that are commonly added together. | e.g., `SpriteBundle`. |
| **App** | The main Bevy structure that holds the ECS World, Schedules, and Plugins. | |
| **Crate** | A Rust package (library or binary). OPTS is a workspace of multiple crates. | |
| **Workspace** | A Cargo feature that allows multiple crates to share a single `Cargo.lock` and `target` directory. | |
| **`nom`** | A parser combinator library for Rust, used for parsing text formats. | Used for `.sco`, `.sli`, `.hof`. |
| **`binrw`** | A Rust crate for reading and writing binary data, used for parsing `.o3d`. | |

---

## 🚏 Public Transport & Simulation

| Term | Definition | Notes |
| :--- | :--- | :--- |
| **GTFS** | **General Transit Feed Specification.** A standard format for public transport schedules and geographic data. | A future target for OPTS. |
| **Route** | A specific line of public transport (e.g., "Bus 42" or "Tram 12"). | |
| **Trip** | A single journey of a vehicle along a route at a specific time. | |
| **Stop** | A designated place where passengers board or alight. | |
| **Dwell Time** | The amount of time a vehicle spends stopped at a station or stop. | |
| **Headway** | The time interval between consecutive vehicles on the same route. | |
| **Deadheading** | The movement of a vehicle without passengers, typically to or from a depot. | |
| **Depot** | A facility where vehicles are stored, maintained, and dispatched from. | |
| **RAPTOR** | **Round-Based Public Transit Routing.** An algorithm for finding the fastest path through a transit network. | Planned for `opts-sim`. |

---

## 📐 Coordinate Systems & Math

| Term | Definition | Notes |
| :--- | :--- | :--- |
| **Z-up** | A coordinate system where the Z-axis points upwards. Used by OMSI. | |
| **Y-up** | A coordinate system where the Y-axis points upwards. Used by Bevy. | Requires conversion. See coordinates.md *(planned; blocked on mutation tests, see the [roadmap](roadmap/README.md))*. |
| **Right-Handed** | A coordinate system where the cross product of X and Y yields Z. | Bevy uses this. |
| **Left-Handed** | A coordinate system where the cross product of X and Y yields -Z. | OMSI uses this. |