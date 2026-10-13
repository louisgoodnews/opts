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
- [ ] ADR-0010 + charter `architecture/hardware-probe.md`
- [ ] crate skeleton, Fact, profile schema
- [ ] probes (common, linux, macos, windows, other)
- [ ] benchmarks + store
- [ ] recommendations + tests
- [ ] generated field reference + test
- [ ] redaction / no-network / budget tests
- [ ] opts-tools hardware report
- [ ] control protocol hardware.profile
- [ ] settings registry Advanced -> Hardware page, de/en
- [ ] CI macOS job, Windows tests
- [ ] cross-target checks
- [ ] docs: M-0081, ROADMAP, BOARD, R-22, SUMMARY, crate graph
- [ ] gates
