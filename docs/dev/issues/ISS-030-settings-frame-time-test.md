# ISS-030: Settings-screen frame-time test is load-sensitive

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-030 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (test hygiene) |
| **Milestone** | [M-0063](../../src/roadmap/phase-9/M-0063.md) Settings registry (the page under test); the frame budget is from [M-0088](../../src/roadmap/phase-9/M-0088.md) |
| **Status** | in progress 2026-11-22 on `agent/settings-test-hygiene` |

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
