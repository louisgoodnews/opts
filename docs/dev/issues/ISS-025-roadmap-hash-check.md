# ISS-025: Done board items cite no commit hash

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-025 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (records) |
| **Milestone** | none of its own: a records fix to [`roadmap_audit.py`](../../../scripts/roadmap_audit.py) and the board |
| **Status** | done 2026-11-22 (rule in `bb72edfa`; closure on `agent/records-2`). 39 of the 56 legacy lines cite the commit that delivered them; 17 stay explained in `roadmap-audit-known.txt`, with the reasons below. |

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
- Open, and explained: 17 legacy lines (below). Their hashes could not be found in git; none is invented.

## Outcome 2026-11-22 (`agent/records-2`)

- **Legacy lines.** Each unhashed `[x]` line was searched by its milestone id, its branch and its subject or date (`git log master`). A commit was taken only where its subject or the line names the same item, and it is an ancestor of master. Choices of note: a merge named by the line's branch counts where the line cites that branch; a feature commit counts where the line names the file or item it added. 39 lines now cite a hash.
- **Audit.** Before: 61 findings, all explained (56 H01, 3 R02, 1 B02, 1 F01), exit 0. After: 22 findings, all explained (17 H01, the same 5 others), exit 0. 39 known entries removed.
- **Board.** The combined records line is split: ISS-025 `[x]` (this issue) and ISS-027 `[~]` ([issue](ISS-027-m0124-evidence.md)).

### Leftover legacy lines (17, still in `roadmap-audit-known.txt`)

| Board line | Why no hash is cited |
| :--- | :--- |
| Implement terrain elevation mesh tessellation (M-0025) | `build_terrain_mesh` was moved by later M-0151 splits; the commit that introduced it was not found by subject. |
| Fix rustdoc links to removed Bevy bundles (M-0040) | Says "done with the CI gate fix, 2026-10-22"; no commit of 2026-10-21 to 23 names the rustdoc or CI fix. The rustdoc fixes found are of 2026-11-16. |
| Fix the CI gate failures at HEAD (M-0052/M-0040) | As above: the 2026-10-22 gate fix commit was not found. |
| Preflight, CPU rasteriser, metrics and verdicts (M-0054) | Phase B, done 2026-10-12; no commit names these items. |
| Missing-texture sentinel and diagnostics (M-0054) | Phase B; the nearest commit (`41a13ed6`) fixes texture folders, a different item. |
| Measure 1,000-object sample; calibrate thresholds (M-0054) | A measurement; no commit names the calibration. |
| Decide the owner of the scenery-script runtime (M-0054/M-0026) | A decision, recorded in a spec (`089e0a6d` specifies the runtime); no commit is the decision. |
| Köln, Verteilerkreis: "6 tiles" warning (M-0025/M-0090) | The ground-paint part is `3b1afb3b`; the warning has no commit. Not claimed. |
| `target/` hygiene note (M-0052) | `CONTRIBUTING.md` note; no commit names it. |
| Profile of Spandau, Bremen and Köln drives | Measurements, no commit. |
| Hitches while driving (compose_alpha) | The fix commit was not found by subject. |
| Vehicle mirrors showed the static picture (pooled material) | The grouping fix was not found by subject. |
| Render fixes from the 2026-10-22 field report (`agent/vehicle-systems`) | Several items in one 73-commit branch; no single commit. |
| Key layouts per kind of vehicle (M-0142 addendum) | Not found; `0822e193` is the key registry, a different item. |
| Test drives 2026-10-22 | Observations from a session; no commit. |
| AI traffic drives into a standing bus (M-0049/M-0103) | Several items on `agent/ai-rules` and `agent/crossing-arbiter`; no single commit. |
| Crate architecture: legacy vs. native separation (M-0131 to M-0134) | Multi-milestone design line; no single commit. |

## Done when

- `python3 scripts/roadmap_audit.py --check` exits 0 with the 17 H01 explanations; a new `[x]` line without a hash fails (unit test `a_resolved_hash_passes_a_missing_one_fails`, and an invented hash `an_invented_hash_fails`). **Met.**
- Each leftover line is explained above. **Met.** The leftover lines can close later by adding their commit and removing the entry.
