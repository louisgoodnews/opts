# ISS-075: Trailer-front seam (pitch and roll) and the trailers the data does not yet support

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-075 (Phase 4 status audit, 2026-11-26) |
| **Severity** | major (player; visible on articulated buses) |
| **Milestone** | [M-0053](../../src/roadmap/phase-4/M-0053.md) Articulated Vehicle Completion |
| **Status** | open, reported 2026-11-26 (Phase 4 status audit) |

## Problem

The trailer-front joint seam had its yaw, render pivot and sim elevation fixed on `master`
(`17473f64`, `914edeec`, `3f614abe`). Two things stay open:

1. The seam is verified across a 48-case sweep on `master` (`6ab7ff80`), but pitch and roll are not
   recorded as verified in the articulation card, and no in-game check across speeds is recorded.
2. `[couple_front]` (reversed units) and parked trailers have no data path in the installation
   (decision 2026-10-31 on evidence). Their open state must stay visible.

The roadmap cited ISS-011 and ISS-037 for the seam. Neither issue file exists in `docs/dev/issues/`,
so those references pointed nowhere; this issue replaces them.

## Evidence

- `docs/src/roadmap/phase-4/M-0053.md:93`: `[couple_front]` and parked trailers, "no data path, stays
  open on evidence".
- `docs/dev/BOARD.md` (M-0053 block): seam sweep passes on master (`6ab7ff80`); "pitch and roll open".
- `ls docs/dev/issues/ISS-011*` and `ISS-037*`: no file.

## Next

1. Record pitch and roll in the sweep with numbers, or extend the sweep to cover them.
2. Run an in-game check at two speeds on a trailer bus and record it in M-0053.
3. Keep `[couple_front]` and parked trailers closed on evidence unless new data is found; record the
   search that was done.

## Done when

- The pitch and roll seam has recorded numbers within the tolerance stated in the articulation spec.
- M-0053 says where the parked-trailer and `[couple_front]` search ended.
