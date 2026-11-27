# ISS-034: Structure ratchet has no total-line budget

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-034 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (code quality) |
| **Milestone** | [M-0147](../../src/roadmap/phase-15/M-0147.md) Structure Audit v2 and the ratchet gates (the ratchet this extends); the file limit is [M-0151](../../src/roadmap/phase-15/M-0151.md) |
| **Audit** | Project audit 2026-11-18, architecture 4 (PRJ-14) |
| **Status** | superseded 2026-11-27 by the owner: the per-crate total budget is dropped (`docs/dev/crate-budget.txt` removed, `check_crate_budget` removed from `scripts/structure_audit.py`). The owner's rule: no crate size gate; splitting into many crates waits until v1.0.0. The per-file and per-function ratchets stay. |

## Problem

The 2026-11-18 audit found that `structure_audit.py check` passes, but every ratchet is per file or per function. A crate can grow without limit as long as each file stays under 150 code lines: 7 files sit at exactly 150 code lines (about 220 in total), and the largest functions (`update_clouds`, `build`, `compute_neighbours`) are 80 lines. Nothing bounded the size of a crate.

## Work

1. A budget file, `docs/dev/crate-budget.txt`, with one line per crate: `<crate> <code lines>  # <reason>`. The budget is the code-line total of the crate's `src/` (the figure `crate_table` already reports).
2. `check_crate_budget` in `scripts/structure_audit.py` fails when a crate is over its budget, or has no entry. A budget is only lowered by hand; `seed` does not write it, so `seed` cannot absorb growth.
3. The budgets are set to the totals on 2026-11-22 (opts-game 91,320; opts-bevy 45,877; 17 crates, 211,399 in all). No per-file or per-function ratchet was raised.
4. `scripts/test_structure_audit.py` has `test_the_total_budget_per_crate_fails_on_growth_and_on_a_missing_entry`: a one-line growth fails with `over its total budget of`, a missing entry fails with `not in docs/dev/crate-budget.txt`, and the tree passes at the budget.
5. The spec is in [setup.md](../../src/contributing/setup.md) (the structure step).

## Checks

- `python3 scripts/structure_audit.py check` exits 0 on this tree (no FAIL line).
- `python3 scripts/test_structure_audit.py`: 12 tests, `OK`.
- Removal check (2026-11-22): the call `bad += check_crate_budget(files)` was disabled; `python3 scripts/test_structure_audit.py Fixture.test_the_total_budget_per_crate_fails_on_growth_and_on_a_missing_entry` gave:
  `AssertionError: 'FAIL demo:' not found in 'FAIL crates/demo/src/lib.rs: grew from 397 to 398 lines of code (it is over the 150-line limit; known files may only shrink)'`, `FAILED (failures=1)`. The call was restored from a saved copy, `grep` for the disabled marker returns 0, and the same test prints `OK`.

## Open

- The budgets are a ceiling, not a target. The "target 40 lines on the top 20 functions" in the audit is not started; it belongs to the function ratchet (M-0150).
- `opts-game` (48% of the lines) is outside the 150-line ratchet of ISS-032; the budget only stops it growing.
