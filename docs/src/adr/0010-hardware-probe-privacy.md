# ADR-0010: Hardware probe: capability facts only, local only, no identifiers

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0010 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-13 |
| **Deciders** | Project lead, maintainers |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

Defaults such as the tile radius ([M-0080](../roadmap/phase-4/M-0080.md)), texture filtering, look and UI scale depend on the machine. Guessing from model names is brittle; asking the user is noisy. The game must learn what the machine can do, but a hardware probe is also the first step of every fingerprinting scheme, and OPTS is an open-source community project that promises no telemetry. The project lead asked for "deep research of the hardware **without spying on the system**" (field report R-22).

## 🎯 Decision

**We will add a crate `opts-hardware` that collects only capability facts needed for a decision, keeps them on the machine, and contains no network code.**

The rules are the [privacy charter](../architecture/hardware-probe.md): no serial numbers, MAC addresses, user or host names, installed-software lists, file listings, network configuration, location or any persistent unique identifier; only coarse classes and rounded numbers; every unknown value is an explicit `Unknown(reason)`; reads of `/proc` and `/sys` use an allowlist of paths; no telemetry; the profile reaches a bug report only after it is shown and the user ticks a box that is off by default. The rules are enforced by tests: a redaction scan of serialised profiles, a dependency check that fails when an HTTP, socket or TLS crate enters the dependency tree, and a generated field list that must match the code.

The crate builds and works on every Linux (glibc and musl, no distribution-specific paths or tools, no root, no systemd/udev/dbus), on macOS and on Windows. Platform code lives behind `cfg(target_os)` modules with one function surface; every probe fails soft.

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **Trust** | A project that reads hardware must be able to prove what it reads; a small audited surface and generated field list make that checkable. |
| **Decision quality** | Measured and classified capability (VRAM, cores, free RAM, storage class) beats model-name tables. |
| **Testability** | Pure functions from a profile to a default are testable on any machine with synthetic profiles. |

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **Probe inside `opts-game`** | Mixes engine code with a privacy-sensitive surface; not auditable on its own, not usable by `opts-tools`. |
| **Collect full model names and driver strings** | Higher fingerprinting value, no extra decision value; classes suffice. |
| **Ask the user for a quality preset only** | Wrong first-run defaults; the memory guard of M-0080 needs real numbers. |
| **Hardware database lookups online** | Network access is forbidden by this ADR. |

## 🌊 Consequences

### ✅ Positive
- Defaults adapt to the machine; the charter is enforceable in CI.
- The same profile feeds the settings page, the control protocol and `opts-tools hardware report`.

### ⚠️ Negative
- Coarse classes mean some edge cases get a conservative default.
- macOS and Windows code paths cannot be exercised on real hardware by the maintainer (see the support matrix in the charter); they are covered by CI builds and platform-independent parser tests only.

### 🔄 Neutral
- A new member of the workspace and a new dependency (`sysinfo`, restricted to the `system` and `disk` features).

## 🔗 References

- [Privacy charter and field list](../architecture/hardware-probe.md)
- [M-0081](../roadmap/phase-5/M-0081.md), [M-0080](../roadmap/phase-4/M-0080.md)
- [ADR-0002: Clean-Room Policy](0002-clean-room-policy.md)
