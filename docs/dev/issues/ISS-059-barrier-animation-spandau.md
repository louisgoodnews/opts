# ISS-059: Barrier animations are broken (Berlin Spandau, Btfh Indira-Gandhi-Strasse)

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-059 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | to be triaged |
| **Status** | open, reported 2026-11-24 |

## Problem

The barrier animations at the Berlin Spandau scene (Btfh Indira-Gandhi-Strasse) are broken (report 2026-11-24, with a screenshot; the screenshot is not stored in the repo). The barrier arms do not animate correctly.

## Next

1. Reproduce at the scene and record which barrier objects and which script variables drive the arms.
2. Check the scenery script for the barrier (the animation variables and their state) and where the game applies them.
3. Fix and add a test on the animation state, or record the owner decision if the behaviour is intended.

## Observation run (2026-11-24, branch `agent/iss059-observe`, no code change)

- Build: `opts-game` and `opts-cli` at master `6ca19123`, debug, built clean (8 m 53 s).
- Run: started 18:37:51, game ready 18:38:19, shut down about 18:40. Map Berlin-Spandau, bus MAN NLC 12C ZF, `--traffic 0 --pedestrians 0`, simulation at 1x, not paused. Game sim time reached about 93 s.
- Result: **the barrier arms were not observed.** The start pose (the bus at Westerwaldstr./Siegener Str.) and six free-camera poses around it (forward 14 to 30 m, right and left offsets, one looking back, two raised to 8 to 12 m) show no barrier. The report screenshot is not in the repo, so its pose is unknown.
- Gap in this issue: the barrier object IDs (`BUE_Str_der_Jugend.sco`, 3215389, `Hs64_Schranke_R/L.sco`), the script variables and the three causes were not in the file this run read. The control protocol has no entity listing in this build (`entity.list` answers 1007), so the crossing cannot be found by name. Next: take the barrier's position from the map data (or the report) and set the observation pose from it.
- Screenshots in `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/iss059-observe/shots/` (`probe_start`, `scan_*`, `hi_*`).
