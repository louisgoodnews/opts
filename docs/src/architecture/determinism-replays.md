# Determinism & Replays

| Metadata | Value |
| :--- | :--- |
| **Status** | 🚧 Planned — not yet written |
| **Last Updated** | 2026-10-14 |

## 📖 Scope (planned)

This page will state what "deterministic" means for OPTS and which parts of the simulation must meet it: the fixed-step loop (default 64 Hz, `--physics-hz`), which state is a function of the fixed step only (vehicle simulation, trailer kinematics, script variables) and which is allowed to depend on the frame rate (rendering interpolation), RNG isolation (seeded generators per vehicle and per traffic system), the platform assumption (same build and machine, or bit-identical across platforms — undecided, see [Articulation Physics](articulation-physics.md) Q-ART-10), and what a replay or a saved session records (inputs versus state). Until it exists, the claims about determinism in [vehicle-scripts-route.md](vehicle-scripts-route.md) are intentions, not guarantees; [Articulation Physics](articulation-physics.md) lists the known places where the code does not meet them yet.
