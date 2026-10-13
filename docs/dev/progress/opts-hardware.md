# opts-hardware (feat/opts-hardware)

Goal: M-0081. Worktree `opts-wt-hardware`, branch `feat/opts-hardware`. Resume from the checklist.

## Design decisions
- Crate deps: serde, serde_json, `sysinfo` (features `system`, `disk` only). No `wgpu` dependency: the renderer hands a
  `GpuInput` / `DisplayInput` / `AudioInput` / `InputDevicesInput` to the probe (plain data); without it those facts are
  `Unknown(NeedsRenderer)`. Linux fallbacks read an allowlist of /proc and /sys paths.
- `Fact<T>`: `Known(T)` or `Unknown(Reason)`; reasons are an enum (no free text, so no identifiers can leak).
- Platform code behind `cfg(target_os)` modules with one function surface (`probe::platform`).
- Probe budget 500 ms: stages check a deadline and mark the rest `Unknown(Timeout)`.

## Checklist
- [x] ADR-0010 + charter `architecture/hardware-probe.md`
- [x] crate skeleton, Fact, profile schema
- [x] probes (common, linux, macos, windows, other)
- [x] benchmarks + store
- [x] recommendations + tests
- [x] generated field reference + test
- [x] redaction / no-network / budget tests
- [x] opts-tools hardware report
- [x] control protocol hardware.profile
- [x] settings registry Advanced -> Hardware page, de/en
- [x] CI macOS job, Windows tests
- [x] cross-target checks
- [x] docs: M-0081, ROADMAP, BOARD, R-22, SUMMARY, crate graph
- [ ] gates

Notes: cross-target clippy -D warnings clean for x86_64/aarch64 windows-msvc, x86_64/aarch64 apple-darwin, linux musl, aarch64 linux gnu (targets installed via rustup, network available).

Open / not done: report-dialog preview with checkbox (switch adds profile text to the saved report only), GPU benchmark, audio channel layout and wheel detection (need audio/input layers). Gates running: see final report. Game test run: `cargo test -p opts-game --lib -- hardware settings` 49 ok.
