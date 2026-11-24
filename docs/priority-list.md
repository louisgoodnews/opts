# Priority list

Status: 2026-11-24, after the merges of `agent/m0123-feedback`, `agent/gui-keyboard-flaky` and
`agent/m0122-run` on master (`438d4d2c`), and the board sync (`41e44e8a`). The board ([BOARD.md](dev/BOARD.md)) keeps the detail;
this page is the order in which the open work should be taken.

## 1. Owner decisions (nothing can move until these are made)

| # | Decision | Options | Recommendation | Source |
| :-- | :--- | :--- | :--- | :--- |
| 1 | ~~Tunnel threshold~~ decided 2026-11-24: keep 15 m; the 95 % recall criterion stays open | (decided) | see M-0115 Notes | [decision](src/roadmap/phase-11/M-0115.md) |
| 2 | Loaded-bus minimum margins (M-0124 criteria 2 and 3) | minimum extra distance in 10 s (measured 23.8 m, 26.6 %); minimum extra stop from 50 km/h (measured 3.3 m, 24 %); store an empty-bus baseline or not | set the minimums, then the tests can pass or fail | [margin tests](src/roadmap/phase-12/M-0124.md) |
| 3 | Is loaded braking 19 % softer than empty at 100 km/h intended (rear axle tyre limit)? | yes, intended; no, fix the brake model (M-0073) | decide with the mass model owner | M-0124 Notes |

## 2. Open verifications and ready work (take in this order)

1. **ISS-038 follow-up: trailer exhaust on rail consists.** Rail trains use `VehicleComposition`,
   not `ArticulatedChain`, so a rail follower's outlet is never read. Needs a design choice and a
   test. Source: [ISS-038 register](audit/issues-2026-11-18.md).
2. **M-0122 graphics: frame times on an idle machine.** The loaded run (`84f608d4`, merged) does not
   separate the bloom settings from the run-to-run spread, and the 16.7 ms criterion is not met.
   Rerun the baseline and bloom off/low/natural/high at fixed quality with the machine idle, before
   judging bloom or AO. Card: [M-0122](src/roadmap/phase-12/M-0122.md).
3. **M-0123 on-screen check.** The refusal toast and the `refused` field are tested and merged
   (`438d4d2c`); the F12 toast has not been seen in the game. Card: [M-0123](src/roadmap/phase-12/M-0123.md),
   removal record: [m0123-refusal-report](dev/removal-checks/m0123-refusal-report.md).
4. **Control server browser check (ISS-002).** Still open; see the board.
5. **Fails-on-parent rule for the trailer sweep (ISS-022, ISS-023).** Still open; see the board.

## 3. Recently merged (for reference)

- `agent/gui-keyboard-flaky` (`706e25d5`): the combobox pick no longer depends on the order of the
  focus and activation systems; the combobox tests moved to `keyboard_combobox.rs`.
- `agent/m0123-feedback` (`438d4d2c`): a refused refuel shows a toast on F12 and sets `refused` in the
  `vehicle.fuel` answer (protocol 1.37.0); removal checks recorded.
- `agent/m0122-run` (`84f608d4`): the M-0122 frame-time measurement note.
- `agent/iss038-verify` (`d43c1cf5`): the trailer's exhaust emitter is read when its variables are
  named `auspuff`, the German for exhaust pipe.

## 4. Operations

- The main checkout's `target/` was cleared on the owner's instruction (2026-11-24). `/home` had
  about 36 GB free afterwards. Agents build in the scratch disk, `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/target-featuregap`.
- Shared build artefacts can be stale across worktrees. If a test reports an old value (for example
  an old protocol version), touch its sources and rebuild before concluding anything.
- Timing-sensitive tests (the settings frame-time check) can fail under load. Rerun before treating a
  failure as real.
