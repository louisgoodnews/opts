# `opts-hardware`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L0 · Foundation (no internal OPTS dependency) |
| **Role** | Privacy-respecting hardware capability probe and default recommendations |
| **Kind** | Library (`opts_hardware`) |
| **Charter** | [Hardware Probe](../src/architecture/hardware-probe.md) · [ADR-0010](../src/adr/0010-hardware-probe-privacy.md) |
| **Status** | 🟢 Implemented (Linux verified; macOS and Windows compiled in CI) |
| **Last Updated** | 2026-11-15 |

## 🎯 Purpose

The crate answers one question for the rest of the game: what can this machine do, and what defaults follow from that. It collects only capability facts (classes, counts, sizes, feature flags), never identifiers, and it contains no network code. Every fact is a `Fact<T>`: either `Known(value)` or `Unknown(Reason)`, so a missing value is explicit and never a guess. Recommendations fall back to a middle-of-the-road answer when facts are unknown.

The crate is a library. It has no windowing, graphics or audio dependency: the renderer, audio and input layers hand their facts in as plain data (`ProbeInputs`), and strings such as adapter names and driver versions are classified and dropped.

## 🔎 What It Probes

`probe()` / `probe_with(&ProbeInputs)` fills a `HardwareProfile` within a time budget (`DEFAULT_BUDGET`, 500 ms). A stage that would start after the deadline is skipped and its facts are `Unknown(Timeout)`. Platform differences live behind one `PlatformProbe` trait with one module per OS (`linux`, `macos`, `windows`, `other`).

| Group | What is recorded | Source |
| :--- | :--- | :--- |
| OS | OS family; kernel version class on Linux, OS version class elsewhere; display server; environment (container or VM hints) | `sysinfo`; Linux reads allowlisted `/proc` and `/sys` files |
| CPU | architecture, vendor class, physical and logical core counts, ISA feature flags (x86_64 and aarch64), L2 and L3 cache sizes | `sysinfo`, `std::arch` detection; caches from sysfs on Linux only |
| Memory | total and available RAM in MiB; the cgroup limit when one is set | `sysinfo` |
| GPU | vendor, device class, backend, VRAM, texture and buffer limits, a short list of known wgpu features, ray tracing and DLSS flags | Renderer-supplied `GpuInput` when given; otherwise the platform fallback (Linux reads DRM vendor and VRAM files) |
| Display | count, width, height, refresh rate, scale, HDR | Renderer-supplied `DisplayInput`; otherwise the Linux DRM connector status and preferred mode |
| Audio | output device present, channel layout | `/proc/asound/cards` on Linux; channel layout only from the audio layer |
| Input | gamepad present, wheel present | Linux `/proc/bus/input/devices` (`js*` handlers only); otherwise the input layer |
| Storage | SSD or HDD class and free GiB of the disk holding the content folder (or the current folder) | `sysinfo` |
| Benchmarks | not run by the probe (`NotRequested`); see below | — |

Where a platform has no source for a fact, the value is `Unknown` with the reason, not a default. The crate starts no process and reads no file outside its allowlist.

## 🧪 Benchmarks and Cache

- `bench::run` is an optional, on-request micro-benchmark for CPU, memory and storage. It is cancellable through a `CancelToken`, shows progress, and uses only `std`. Its units are arbitrary but comparable on the same class of machine. Only `opts-tools` calls it.
- `store` keeps the profile as plain JSON (`hardware.json`) in a folder the caller chooses. Writes go through a temporary file. `with_cached_bench` reuses cached benchmark results only while `HardwareProfile::class_hash()` (a coarse, bucketed hash of the hardware class) still matches.

## 📐 Recommendations

`recommend` holds pure functions from a profile to defaults, with no I/O: GPU tier, the default tile radius for a map (`recommended_tile_radius`), texture filtering, look, UI scale, DLSS availability, a safe resident-memory figure, virtualisation detection, and the load-all memory estimate (`estimate_load_all`). Their results are the stored values of the game settings. `report` renders the profile and its recommendations as plain text for the terminal, the settings page and bug reports; it is generated from the serialised form, so a new field appears there without further edits.

## 🛡️ Privacy Guarantees

- **Path allowlist.** `allowlist` is the only table of paths the probe may read, with the reason for each. A path outside it is refused before any I/O (`Reason::Unavailable`). The table refuses `/etc/hostname`, `/proc/self/environ`, network-interface addresses and DMI serial and UUID files.
- **No network.** `tests/no_network.rs` fails if a networking crate appears in the dependency graph.
- **No identifiers.** `tests/redaction/main.rs` checks that serialised output contains no MAC address or UUID-shaped value, and that every string value belongs to a closed vocabulary of allowed words.
- **No unsafe code.** The crate is `#![forbid(unsafe_code)]`.
- **Field inventory.** `reference` checks the field table against the serialised profile and renders [Hardware Fields](../src/reference/hardware-fields.md). `UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware reference` rewrites that page.

## ✅ Testing

- `tests/budget_and_schema.rs`: the full probe stays within the budget; an exhausted budget gives `Timeout` reasons; a schema round trip; unknown fields are ignored and missing ones are unprobed; a newer schema or garbage input is refused; the class hash is stable and coarse.
- Unit tests in `parse`, `recommend`, `allowlist`, `platform` and `reference`, including a test that real probes never panic on the test machine.

## 📦 Dependencies

`serde`, `serde_json` and `sysinfo` (features `system` and `disk` only). No internal OPTS crate.

## 👥 Users

| Crate | Uses |
| :--- | :--- |
| `opts-game` | `probe_with`, `store` (load, save, path), `report`, `recommend` (tile radius, memory verdict, UI and texture defaults) and the profile types in the settings and new-game screens |
| `opts-tools` | `probe_with`, `bench::run` and `report::render_text` for the `hardware` command |

## 🔗 Related

[Crate Graph](../src/architecture/crate-graph.md) · [Hardware Probe](../src/architecture/hardware-probe.md) · [Hardware Fields](../src/reference/hardware-fields.md) · [ADR-0010](../src/adr/0010-hardware-probe-privacy.md)
