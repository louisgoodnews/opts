# Hardware profile fields (generated)

Generated from `crates/opts-hardware/src/reference.rs` by the test `reference_page_is_up_to_date` (`UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware reference`). Do not edit by hand. The rules behind this list are the [privacy charter](../architecture/hardware-probe.md) ([ADR-0010](../adr/0010-hardware-probe-privacy.md)).

Schema version **1**. Every field below is `known` (a value) or `unknown` with one of the reasons at the end. `probe_ms` (how long the probe took) and `schema` complete the document.

## Fields

| Field | Value | Used for | Source |
| :--- | :--- | :--- | :--- |
| `os.family` | enum | feature gates, DLSS | compile-time OS constant |
| `os.version` | major.minor numbers | bug reports, workarounds | `sysinfo` (kernel on Linux), digits only |
| `os.display_server` | enum | window and input behaviour | Linux: session variables (presence); macOS/Windows: fixed |
| `os.environment` | enum | conservative defaults in VMs and containers | Linux: cgroup keywords, marker files, hypervisor flag, DMI keywords |
| `cpu.arch` | enum | feature gates | compile-time constant |
| `cpu.vendor` | enum | report only | `sysinfo` brand classified, string dropped |
| `cpu.physical_cores` | count | tile radius, worker threads | `sysinfo` |
| `cpu.logical_cores` | count | tile radius, worker threads | `std::thread::available_parallelism` |
| `cpu.features` | allowlisted names | SIMD paths | `std` feature detection |
| `cpu.cache_l2_kib` | KiB | report only | Linux: sysfs cache entries; elsewhere unknown |
| `cpu.cache_l3_kib` | KiB | report only | Linux: sysfs cache entries; elsewhere unknown |
| `memory.total_mib` | MiB | tile radius, memory guard | `sysinfo`, container limit applied |
| `memory.available_mib` | MiB | tile radius, memory guard | `sysinfo`, container limit applied |
| `gpu.vendor` | enum | tier, DLSS | renderer vendor id classified; Linux fallback: PCI vendor id; Apple Silicon: by architecture |
| `gpu.device_class` | enum | tier | renderer device type; Apple Silicon: integrated |
| `gpu.vram_mib` | MiB | tier | renderer; Linux fallback: amdgpu sysfs |
| `gpu.backend` | enum | DLSS | renderer |
| `gpu.max_texture_2d` | pixels | texture budget | renderer limits |
| `gpu.max_buffer_mib` | MiB | mesh budget | renderer limits |
| `gpu.features` | allowlisted names | format choices | renderer feature set, filtered by allowlist |
| `gpu.ray_tracing` | bool | DLSS | renderer |
| `gpu.dlss_runtime` | bool | DLSS | renderer |
| `display.count` | count | window placement | renderer; Linux fallback: connected DRM connectors |
| `display.width` | pixels | UI scale | renderer; Linux fallback: preferred mode |
| `display.height` | pixels | UI scale | renderer; Linux fallback: preferred mode |
| `display.refresh_hz` | Hz | frame pacing hints | renderer |
| `display.scale_percent` | percent | UI scale | renderer |
| `display.hdr` | bool | look | renderer |
| `storage.class` | enum | tile radius | `sysinfo` disk kind of the content volume |
| `storage.free_gib` | GiB | cache sizes | `sysinfo` free space of the content volume |
| `audio.device_present` | bool | audio settings availability | audio layer; Linux fallback: sound card count |
| `audio.channels` | enum | audio mixer layout | audio layer |
| `input.gamepad_present` | bool | controls page | input layer; Linux fallback: `js*` handler count |
| `input.wheel_present` | bool | controls page | input layer |
| `bench.cpu_single_mops` | Mops/s | tile radius refinement | micro-benchmark (on request) |
| `bench.cpu_multi_mops` | Mops/s | tile radius refinement | micro-benchmark (on request) |
| `bench.memory_mib_s` | MiB/s | tile radius refinement | micro-benchmark (on request) |
| `bench.storage_read_mib_s` | MiB/s | streaming expectations | micro-benchmark (on request, scratch file deleted) |
| `bench.gpu_fill` | arbitrary | reserved | not implemented (needs a render device) |
| `bench.measured_for_class` | hash | cache validity | FNV-1a of the bucketed hardware class |

## Closed value sets

* `os.family`: `linux`, `macos`, `windows`, `other`
* `os.display_server`: `wayland`, `x11`, `quartz`, `win32`, `none`
* `os.environment`: `bare`, `container`, `vm`
* `cpu.arch`: `x86_64`, `aarch64`, `other`
* `cpu.vendor`: `intel`, `amd`, `apple`, `arm`, `other`
* `gpu.vendor`: `nvidia`, `amd`, `intel`, `apple`, `qualcomm`, `software`, `other`
* `gpu.device_class`: `discrete`, `integrated`, `virtual`, `software`, `other`
* `gpu.backend`: `vulkan`, `dx12`, `metal`, `gl`, `other`
* `storage.class`: `ssd`, `hdd`
* `audio.channels`: `mono`, `stereo`, `surround51`, `surround71`, `other`
* `cpu.features`: `sse4.2`, `avx`, `avx2`, `fma`, `bmi2`, `popcnt`, `avx512f`, `neon`, `aes`, `crc`, `dotprod`
* `gpu.features`: `texture_compression_bc`, `texture_compression_etc2`, `texture_compression_astc`, `indirect_first_instance`, `multi_draw_indirect`, `timestamp_query`, `shader_f16`, `texture_binding_array`, `float32_filterable`, `ray_query`

## Reasons for unknown values

| Reason | Meaning |
| :--- | :--- |
| `not_probed` | not probed (older profile or probe not run) |
| `not_on_this_platform` | no probe exists on this platform |
| `needs_renderer` | supplied by the renderer or audio layer, which was not available |
| `unavailable` | the interface that holds it is absent |
| `unreadable` | the interface could not be read |
| `unparsed` | the content was not understood |
| `timeout` | the probe budget was used up |
| `not_requested` | an optional measurement that was not requested |
| `cancelled` | cancelled by the user |

## Files the Linux probe may read

Nothing else is opened (test `no_allowlisted_path_names_an_identifier`; every read goes through `allowlist::read`). `*` matches one path segment.

| Path | Used for |
| :--- | :--- |
| `/proc/cpuinfo` | the `hypervisor` flag only |
| `/proc/self/cgroup` | container keywords only |
| `/proc/asound/cards` | number of sound cards only |
| `/proc/bus/input/devices` | number of `js*` handlers only |
| `/sys/devices/system/cpu/cpu0/cache/index*/level` | cache level |
| `/sys/devices/system/cpu/cpu0/cache/index*/type` | cache type |
| `/sys/devices/system/cpu/cpu0/cache/index*/size` | cache size |
| `/sys/class/drm/card*/device/vendor` | PCI vendor id, classified |
| `/sys/class/drm/card*/device/mem_info_vram_total` | video memory (amdgpu) |
| `/sys/class/drm/card*-*/status` | connected or not |
| `/sys/class/drm/card*-*/modes` | preferred resolution only |
| `/sys/class/dmi/id/sys_vendor` | hypervisor keyword only |
| `/sys/class/dmi/id/product_name` | hypervisor keyword only |

Existence only: `/.dockerenv`, `/run/.containerenv`. Directory listings (names): `/sys/class/drm`, `/sys/devices/system/cpu/cpu0/cache`.

