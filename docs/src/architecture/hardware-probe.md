# Hardware Probe (`opts-hardware`): Privacy Charter and Design

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Implemented on Linux; macOS and Windows compile-checked in CI, **untested on real hardware** |
| **Milestone** | [M-0081](../roadmap/phase-5/M-0081.md) (field report R-22) |
| **Decision** | [ADR-0010](../adr/0010-hardware-probe-privacy.md) |
| **Code** | `crates/opts-hardware` |
| **Field list (generated)** | [reference/hardware-fields.md](../reference/hardware-fields.md) |

## 1. Charter

The probe learns what the machine **can do**, to choose defaults. It never learns **who or what exactly** the machine is.

1. **Capability facts only.** A fact exists only when a documented decision uses it ([recommendations](#5-recommendations)).
2. **Never collected:** serial numbers, MAC addresses, UUIDs, user names, host names, installed-software lists, file listings (beyond free space of one volume), network configuration, location, EDID data, GPU or CPU model strings, driver strings, any persistent unique identifier.
3. **Coarse values.** Classes (`intel`, `amd`, `apple`, `nvidia`, ...), counts and sizes rounded to KiB/MiB; model names are classified and then dropped.
4. **No network.** The crate has no HTTP, socket or TLS dependency. The test `no_network_dependencies` reads `cargo metadata` and fails when one appears in the dependency tree of `opts-hardware`.
5. **Local only.** The profile lives in memory and in one optional file (`hardware.json` in the OPTS data folder) that the user can read with any text editor and delete (*Settings → Advanced → Hardware → Delete stored profile*, or by hand).
6. **No telemetry.** Nothing is sent anywhere. The *Report an issue* dialog may attach the profile only after showing it to the user and only when *Allow including the profile in reports* is ticked (default off).
7. **Unknown is explicit.** Every field is `Known(value)` or `Unknown(reason)`; there are no silent defaults and no guessed numbers.
8. **Auditable.** The [generated field list](../reference/hardware-fields.md) is produced from the code and a test keeps it current; the redaction test scans serialised profiles for MAC, UUID, host name and user name patterns.
9. **Reads are allowlisted.** On Linux only the paths in the table of §3 are read, none requires root; unreadable or missing files give `Unknown`.
10. **No shelling out.** The probe starts no external program on any platform.

## 2. Profile and schema

`HardwareProfile { schema, os, cpu, memory, gpu, display, storage, audio, input, bench, probe_ms }`; `schema` is an integer (`SCHEMA_VERSION`). Newer files are rejected, older and unknown fields are ignored on load; missing fields load as `Unknown(not_probed)`. `class_hash` is an FNV-1a hash of the *bucketed hardware class* (OS family, core bucket, RAM bucket, GPU vendor and type, storage class): it identifies a class shared by millions of machines, not a machine, and is used only to decide whether cached benchmark results still apply.

Facts the renderer owns (GPU adapter info and limits, monitors, audio output, gamepads) are handed to the probe as plain data (`GpuInput`, `DisplayInput`, `AudioInput`, `InputInput`) so that `opts-hardware` needs no graphics or audio dependency. Without them the facts are `Unknown(needs_renderer)`, except where a Linux fallback exists.

## 3. What each platform probes (support matrix)

**The maintainer has no macOS or Windows machine. Those code paths are verified only by CI builds (and platform-independent unit tests of the parsers); they are untested on real hardware.** Reports from users are welcome.

| Fact | Linux (any distribution, glibc/musl, containers) | macOS (Intel, Apple Silicon) | Windows (x86_64, aarch64) |
| :--- | :--- | :--- | :--- |
| OS family, version class | `std::env::consts`, `sysinfo` (kernel major.minor) | `sysinfo` (major.minor) | `sysinfo` (major.build class) |
| Display server | `XDG_SESSION_TYPE`, `WAYLAND_DISPLAY`, `DISPLAY` (presence only) | `quartz` (fixed) | `win32` (fixed) |
| Container / VM hint | `/proc/self/cgroup` keywords, `/run/.containerenv`, `/.dockerenv` (existence), `/proc/cpuinfo` `hypervisor` flag, `/sys/class/dmi/id/{sys_vendor,product_name}` classified then dropped | unknown | unknown |
| CPU class, cores, ISA features | `sysinfo` + `/proc/cpuinfo`-free: `std::is_x86_feature_detected!`, aarch64 detection | same | same |
| CPU caches | `/sys/devices/system/cpu/cpu0/cache/index*/{level,type,size}` | unknown | unknown |
| RAM total/available | `sysinfo` (honours cgroup limits) | `sysinfo` | `sysinfo` |
| GPU vendor/class/limits | from the renderer (`GpuInput`); fallback `/sys/class/drm/card*/device/vendor` (class only) | renderer only | renderer only |
| VRAM | renderer, or `/sys/class/drm/card*/device/mem_info_vram_total` (amdgpu) | unified memory flag from renderer | renderer only |
| Display | renderer (`DisplayInput`); fallback `/sys/class/drm/card*-*/{status,modes}` (resolution, count) | renderer only | renderer only |
| Storage class, free space | `sysinfo` disks (volume of the content folder) | same | same |
| Audio device present | renderer (`AudioInput`); fallback `/proc/asound/cards` (count only) | renderer only | renderer only |
| Gamepad / wheel | renderer (`InputInput`); fallback `/proc/bus/input/devices` handler words (`js*`) counted | renderer only | renderer only |
| Micro-benchmarks | CPU, memory, storage (std only) | same | same |
| GPU benchmark | not implemented (needs a render device; field stays `Unknown(not_requested)`) | | |

| Platform | Verification status |
| :--- | :--- |
| Linux x86_64 glibc | ✅ built and tested in CI and by the maintainer |
| Linux musl, aarch64, containers | 🔧 same code, generic interfaces only; not built in CI, untested |
| macOS (Intel, Apple Silicon) | 🔧 built and unit-tested in the CI macOS job; **untested on real hardware** |
| Windows x86_64 | 🔧 built and unit-tested in the CI Windows job; **untested on real hardware** |
| Windows aarch64 | 🔧 same source as x86_64; not built in CI, untested |

## 4. Time budget, benchmarks, cache

The whole probe has a budget of 500 ms: stages check the deadline and mark what is left `Unknown(timeout)`. Micro-benchmarks (CPU single and multi thread, memory bandwidth, storage scratch read) run only on request (`opts-tools hardware report --bench`, or *Re-detect* with benchmarks), with a progress callback and a cancel token; results are cached in the local profile file with the class hash and reused while the class is unchanged.

## 5. Recommendations

Pure functions, tested over a table of synthetic profiles (`recommend.rs`):

| Function | Input facts | Result |
| :--- | :--- | :--- |
| `gpu_tier` | GPU device class, vendor, VRAM, software flag | `Low`, `Mid`, `High`, `Unknown` |
| `recommended_tile_radius(profile, map)` | available RAM, GPU tier, logical cores, storage class, map size | integer ≥ 1 (≤ largest useful radius of the map) |
| `recommended_texture_filtering` | GPU tier | `off`, `4`, `8`, `16` |
| `recommended_look` | GPU tier, HDR | `vivid`, `natural`, `bevy` |
| `recommended_ui_scale` | display scale factor, resolution | percent in steps of 25 (75..200) |
| `dlss_available` | GPU vendor and class, backend, ray-tracing flag, OS | `Yes`, `No(reason)`, `Unknown` |

## 6. Where it surfaces

* `opts-tools hardware report [--json] [--bench]` prints the profile in the terminal.
* Control protocol `hardware.profile` returns the profile (and the stored-profile state).
* *Settings → Advanced → Hardware*: the profile, *Re-detect*, *Delete stored profile*, *Allow including the profile in reports* (default off); strings in `messages_{en,de}.ftl`, rows through the [settings registry](settings-registry.md).
