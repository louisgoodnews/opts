# ISS-059: Barrier animations are broken (Berlin Spandau, Btfh Indira-Gandhi-Strasse)

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-059 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (player) |
| **Milestone** | to be triaged |
| **Status** | closed 2026-11-26: not reproduced as a bug. The barrier swing matches the model (delay as a rate, about 0.7 s to half-way; runtime check and a visual check from above show no 0.25 s snap); the earlier snap was an observation artifact. ISS-060 (Falkensee) is fixed separately. |

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

## Observation retry (2026-11-24, branch `agent/iss059-observe`, no code change)

- Build: `opts-game` debug at `a326eaaf` (docs only since `6ca19123`), built clean (1 m 42 s).
- Run: game started 18:44:44, ready 18:45:07, exited 18:46:27. Map Berlin-Spandau, bus MAN 12C ZF, `--traffic 0 --pedestrians 0`, clock 12:00:14 at ready, rate 1x. Sim time covered 18.2 s to 92.4 s (one 55 s cycle and more), 48 screenshots at about 1.6 s spacing.
- Object positions from the tile text (`tile_2383_11286.map`): crossing controller 3215389 (object 14) and arms 43 to 46. The absolute OMSI position is the tile corner from the Mercator grid (`opts_types::grid`) plus the local position. Object 14 about (885954.5, 5850771.9), arms 43 and 44 about 14 m and 9 m north, arms 45 and 46 about 15 m and 9 m south, arms spread about 25 m east-west; the arms form a ring of about 12 to 15 m around object 14.
- Pose: camera at (885944.5, 5850771.9), 10 m west of object 14, z 33.5, yaw 90, pitch -8, free camera, fov 45.
- Visibility: **only one arm was in the frame**: a red-white arm on a post below an Andreaskreuz sign at the upper left of the view, about 15 m away, identity by object number not confirmed. Arms 43 to 46 were not all in view; the pose could not frame the whole ring at 8 to 12 m with fov 45. The screenshot is `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/iss059-observe2/shots/pose_check.png`.
- What the visible arm did (sim time, from the screenshot series `obs_000` to `obs_047`):
  - 18.2 s: closed, extended across the road.
  - 19.8 s: already raised, steep, near vertical (open); no intermediate frame, so the swing or snap between 18.2 s and 19.8 s is not seen.
  - 21.4 s to 70.3 s: raised (open). A train passed in front from about 20 s to 33 s (frames 20 to 33 show the train, not the arm).
  - 71.9 s: closed again, horizontal. 73.5 s: partly lowered. 75.1 s: raised again.
  - Period between the two closed states: about 53.7 s (18.2 to 71.9), close to the 55 s game-clock cycle, but the sampling is too coarse to say.
- Not seen: the other three arms; swing axis, snap or stall; the exact close and open times (only the 1.6 s bracket above).
- Cause: not determined from this run. Open question: the closed state was caught only at single samples (18.2 s and 71.9 s), so its hold time is unknown; a finer sample (0.2 s) across 18 to 21 s and 70 to 76 s, with the camera on arms 43 and 45, is needed to tell a snap from a swing and a stall from a short hold.
- Screenshots: `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/iss059-observe2/shots/` (`pose_check.png`, `obs_000.png` to `obs_047.png`); contact sheet of the arm region `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/iss059-observe2/arm_seq.png`.

### Fine sampling of arm 43 (2026-11-24, same branch, no code change)

- Run: game started 19:26:05, exited 19:27:31 (second attempt; the first fine run had no sim time, not used). Same map, bus and traffic settings as above. Camera at the same position, yaw 31.1, pitch -11, aimed at arm 43. Screenshots every about 0.105 s of sim time from sim 5.5 s to 77 s (608 frames, JPEG, sim time from the screenshot metadata).
- Opening: horizontal (closed) at 18.97 s (frame 100), starting to lift at 19.11 s, near vertical at 19.26 s. About 0.3 s for roughly 90 degrees; the arm pivots at its base post, rising to a standing position. No gradual swing was seen in the frames.
- Closed hold before that opening: closed at every frame from the earliest frame (17.6 s) to 18.97 s.
- Closing: not captured. The arm is horizontal at 69.55 s (first frame of the second window), so the closing lies in the span 21 to 69.5 s, which a train covers in the coarse run.
- Second opening: horizontal at 74.48 s (frame 583), starting at 74.59 s, near vertical at 74.69 s. About 0.2 s.
- Closed hold before the second opening: at least 4.9 s (from 69.55 s to 74.48 s).
- Period between the two openings: about 55.4 s (19.1 s to 74.5 s), matching the 55 s game-clock cycle.
- Correction to the coarse readings above: the arm at 73.5 s is horizontal (not partly lowered); the 1.6 s sampling misread it.
- Cause: not determined. The raise is fast (about 0.2 to 0.3 s) with no gradual swing, which fits a snap or a step in the animation rather than a slow swing; the closing transition is not seen, and arms 44, 45 and 46 are not observed.
- Screenshots: `/home/louisgoodnews/nvme2n1/Dokumente/opts-scratch/work/iss059-fine1/shots_fine/` (fine_0000.jpg to fine_0607.jpg); montages `w1_open.png` and `w2_close.png` in the same folder's parent.
