# ISS-030: Settings-screen frame-time test is load-sensitive

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-030 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (test hygiene) |
| **Milestone** | [M-0063](../../src/roadmap/phase-9/M-0063.md) Settings registry (the page under test); the frame budget is from [M-0088](../../src/roadmap/phase-9/M-0088.md) |
| **Status** | done 2026-11-22 on `agent/settings-test-hygiene` |

## Problem

`the_content_page_builds_within_a_frame_and_fills_in_when_the_facts_arrive` (`crates/opts-game/src/settings_screen/tests/content/facts.rs`) fails when the machine is busy. It compares the worst single `app.update()` against an absolute 250 ms limit, while the real budget is 33 ms. The limit is so loose that a quiet machine passes it without showing anything, and a loaded machine can exceed it for reasons unrelated to the page.

## Decision

Relative check, in the default test run. The test measures a baseline frame in the same run (the median of idle frames while the facts are still pending) and asserts that the worst frame during fill-in stays under the larger of ten times that baseline and the 33 ms budget. Load slows both measurements, so the ratio stays meaningful; the 33 ms floor keeps the quiet-machine check at the real budget.

Rejected: a budgeted test ignored unless an env var is set. It would keep the 33 ms budget but would not run in the default test run, so the frame property would have no coverage in normal use.

## Work

1. Replace the absolute 250 ms limit with the relative check above.
2. Keep the 1.5 s "building the page did not wait for the facts" check and the value assertions unchanged.
3. Show the test passes on a quiet run (numbers in the commit message or the status line below).

## Done when

- The default `cargo test` run of the settings screen passes, with no env var and no `#[ignore]`.
- The test states what it measures (frame time against the run's own baseline, with the 33 ms floor) in its doc comment.
- The board line and this file's status are updated on completion.

## Outcome (2026-11-22, `agent/settings-test-hygiene`)

- **What the check measures now.** The worst single frame of the content page while the facts are gathered, compared with the run's own baseline: the median of 5 idle frames taken while the facts are still pending. The limit is `max(10 x baseline, 33 ms)`. The 33 ms floor is the real budget, so on a quiet machine the check is the budget itself; under load both the baseline and the frames slow down, so the ratio holds. The 250 ms absolute limit is gone. The 1.5 s "did not wait for the facts" check and the value assertions are unchanged.
- **Runs.** Clean build of `-p opts-game` first (`cargo clean -p opts-game`, exit 0), then `cargo test -p opts-game --lib settings_screen::tests::content::facts` three times through `cargo-lowmem.sh`:
  - run 1 (load 3.3, quiet): `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 756 filtered out; finished in 3.11s`
  - run 2 (load 10.3): `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 756 filtered out; finished in 3.10s`
  - run 3 (load 10.0): `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 756 filtered out; finished in 3.10s`
  The two runs at load about 10 were under another agent's build, so the relative check held under load too. The test prints its numbers only on failure, so the exact per-run frame times were not recorded.
- **Limits.** Three runs on one machine; the worst-frame figures are not recorded in the output. Not run in the game. The limit is 33 ms whenever the idle baseline is below 3.3 ms; above that it grows with the baseline (10 x). So a loaded run gets a larger limit, which is the intent: it is relative to that run's own idle frames, not a fixed number. The check does not prove the page is within budget on a loaded machine; it proves the fill-in frames are not out of proportion to idle frames.
