# ADR-0001: Why Bevy and Rust?

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0001 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-03 |
| **Deciders** | Project Lead |
| **Supersedes** | None |

---

## 📌 Context

OPTS (Open Public Transport Simulator) is a new, open-source project that aims to natively load and simulate OMSI 2 assets. Because OMSI 2 is a proprietary, aging engine (released in 2011), we cannot legally or technically extend it. We must build a new simulator from scratch that reads OMSI's file formats.

This presents a unique set of requirements:

1.  **Heavy Asset Processing:** We must parse complex binary (`.o3d`) and text (`.sco`, `.sli`) formats.
2.  **Real-Time Simulation:** The simulator must handle vehicle physics, passenger AI, and large, detailed maps without frame drops.
3.  **Long-Term Maintainability:** As an open-source project, the codebase must be safe, modular, and easy for new contributors to understand.
4.  **Cross-Platform:** The simulator should run on Windows (the primary OMSI platform), Linux, and macOS.
5.  **Legal Safety:** We must avoid proprietary engines or SDKs that could complicate licensing or distribution.

---

## 🎯 Decision

We will build OPTS using the **Rust** programming language and the **Bevy** game engine.

Specifically:
- **Language:** Rust (Edition 2021)
- **Engine:** Bevy (latest stable version)
- **Architecture:** Entity Component System (ECS) provided by Bevy

---

## 🧠 Rationale

### Why Rust?

| Reason | Explanation |
| :--- | :--- |
| **Memory Safety** | Rust's ownership model eliminates entire classes of bugs (null pointers, buffer overflows, data races) that are common in C++ game development. This is critical for a community project where code review resources are limited. |
| **Performance** | Rust offers performance comparable to C and C++, which is essential for processing large OMSI maps and running complex simulations. |
| **Excellent Tooling** | Cargo is arguably the best package manager and build system in any language. It makes workspace management, dependency resolution, and testing trivial. |
| **Parsing Ecosystem** | Rust has world-class libraries for parsing. `nom` (parser combinators) and `binrw` (binary reading/writing) are perfect for the OMSI formats. |
| **Cross-Platform** | Compiling Rust to Windows, Linux, and macOS is straightforward with minimal platform-specific code. |
| **Concurrency** | Rust's `rayon` library makes data-parallel processing safe and easy, which will be vital for loading maps and simulating passengers. |

### Why Bevy?

| Reason | Explanation |
| :--- | :--- |
| **Pure Rust** | Bevy is written entirely in Rust. This means we can use a single language for the entire stack (parsing, simulation, rendering, UI). There is no C++ interop layer to maintain. |
| **ECS Architecture** | Bevy's Entity Component System is a natural fit for a simulator. Vehicles, passengers, bus stops, and routes are all naturally represented as entities with components. Systems handle the logic cleanly. |
| **Modern Renderer** | Bevy uses `wgpu`, a modern, cross-platform graphics API that supports Vulkan, Metal, DirectX 12, and WebGPU. This ensures future-proof rendering. |
| **Plugin System** | Bevy's plugin architecture aligns perfectly with our crate structure. `opts-bevy` will be a Bevy plugin that can be added to any Bevy app. |
| **Active Community** | Bevy is one of the fastest-growing game engines on GitHub. It has an incredibly active Discord, frequent releases, and a vibrant ecosystem. This is vital for an open-source project. |
| **Validation by Industry** | The developer of OMSI 2 and its successor LOTUS has moved to using Bevy for their next-generation simulator. This validates Bevy's suitability for this exact use case. |

---

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **C++ with Unreal Engine** | Unreal is powerful, but its C++ build times are notoriously slow, its codebase is massive, and its licensing model (5% royalty) is problematic for an open-source project. Memory safety is also a major concern. |
| **C# with Unity** | Unity is the most popular engine for indie simulation games, but its recent licensing changes (Runtime Fees) have created significant uncertainty. It also relies on C# and garbage collection, which can cause frame hitches in a real-time simulator. |
| **C++ with Godot** | Godot is a fantastic open-source engine, but using Rust with it requires GDExtension, which adds a layer of complexity and friction. Godot's ECS is also not as mature or idiomatic as Bevy's. |
| **Rust with Fyrox or Macroquad** | Fyrox is a promising Rust engine but is less mature than Bevy and has a smaller community. Macroquad is designed for simple 2D games and is not suitable for a 3D simulator of this scope. |
| **Custom Engine in C++** | Building a custom engine from scratch is a massive undertaking that would distract from the core goal of OPTS: simulating public transport. It also exposes the project to unnecessary security and maintenance risks. |

---

## 🌊 Consequences

### ✅ Positive
- **Single Language:** Rust for parsing, simulation, rendering, and tooling.
- **Memory Safety:** Eliminates a huge class of potential bugs and security vulnerabilities.
- **Modularity:** The Cargo workspace allows strict separation of concerns (`opts-core` has no Bevy dependency).
- **Performance:** Rust and Bevy are fast enough to handle large OMSI maps.
- **Legal Safety:** Both Rust and Bevy are MIT/Apache-2.0 licensed, which is compatible with our open-source goals.

### ⚠️ Negative
- **Bevy is Pre-1.0:** Bevy releases breaking API changes frequently (every 3-4 months). We will need to allocate time to update our code when new versions are released.
- **Compilation Times:** Bevy can take a long time to compile in debug mode. We have mitigated this by optimizing dependency profiles in the root `Cargo.toml`.
- **Learning Curve:** Contributors familiar with Unity or Unreal will need to learn Rust and Bevy's ECS paradigm.

### 🔄 Neutral
- **Community Reliance:** We are dependent on the Bevy community for bug fixes and features. However, we can contribute back to Bevy directly if we encounter issues.

---

## 🔗 References
- [Bevy Official Website](https://bevyengine.org/)
- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [LOTUS Simulator (Bevy-based successor to OMSI)](https://www.lotus-simulator.de/)
- [OPTS Glossary](../glossary.md)