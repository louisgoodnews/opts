# ISS-025: Done board items cite no commit hash

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-025 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (records) |
| **Milestone** | none of its own: a records fix to [`roadmap_audit.py`](../../../scripts/roadmap_audit.py) and the board; the milestones of the lines it touches keep their own cards |
| **Status** | in progress 2026-11-22 on `agent/records-hygiene` (first commit: board and register marked in progress). The check, the hash fixes and the unfixable list follow in later commits on the same branch. |

## Problem

The board rule says every `[x]` line carries a commit hash, so a reader can find the code. The register counted 115 of 165 done lines without one. `roadmap_audit.py` did not check board lines at all.

## Work

To be completed on this branch: the audit rule, the hashes that git can show, and the lines that cannot be fixed.
