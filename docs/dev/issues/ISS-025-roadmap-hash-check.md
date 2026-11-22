# ISS-025: Done board items cite no commit hash

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-025 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (records) |
| **Milestone** | none of its own: a records fix to [`roadmap_audit.py`](../../../scripts/roadmap_audit.py) and the board |
| **Status** | rule and fixes done 2026-11-22 on `agent/records-hygiene`. 19 lines cite a hash git resolves; 56 legacy lines are explained, not fixed (their commits are not in git). |

## Problem

The board rule says every `[x]` line carries a commit hash, so a reader can find the code. The register counted 115 of 165 done items without one. `roadmap_audit.py` did not check board lines at all, so the rule was not enforced.

## Rule (H01)

`roadmap_audit.py` reports H01 for each `- [x]` line of `docs/dev/BOARD.md` that cites no commit hash git resolves. A hash counts only if `git cat-file --batch-check` resolves it to a commit, so an invented or mistyped hash fails like a missing one. The finding is classified like the others: `--check` fails unless it is explained in `docs/dev/roadmap-audit-known.txt`, and an explanation whose line has since changed (its slug is its bold title) is reported as stale. The unit tests are in `scripts/test_roadmap_audit.py`, run from `scripts/local-ci.sh`.

## Result

- The board had 115 `[x]` lines. Before the fixes the audit found 75 without a resolvable hash; after, 56.
- 19 lines got the hash of the commit they describe (the milestone's own completion commit, or the commit named by the line's branch and date). The choice was made from commit subjects; lines where no commit clearly matched were not given one.
- 56 legacy lines are explained under H01 in `roadmap-audit-known.txt`, with their date and branch in the line. Adding the hash later and removing the entry is the way to close each one.

## Done when

- `python3 scripts/roadmap_audit.py --check` exits 0 with the 56 H01 explanations; a new `[x]` line without a hash fails (unit test `a_resolved_hash_passes_a_missing_one_fails`, and an invented hash `an_invented_hash_fails`).
- Open: the 56 legacy hashes. They are not in git under the milestone name or the branch, and no hash is invented for them.
