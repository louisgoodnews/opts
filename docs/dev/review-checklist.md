# 👀 Review Checklist: What Needs a Look in the Running Game

_Written 2026-10-29. Companion to the Review column of [`BOARD.md`](BOARD.md) and to the "verification debt" point of the [gap analysis](../src/meta/gap-analysis-2026-10-29.md)._

Most of the work on this project is checked headlessly (tests, the control protocol, corpus runs). That finds defects but cannot say whether a bus looks and feels right. This page lists, in one place, what is waiting for a person to look at the running game, with the steps and what you should see. Each item takes a few minutes.

**How to use it.** Start a drive (*Plan a drive*, or `opts-game --autostart` with a map and vehicle), try the item, and answer one of:

* **accepted**: I move the board item to *Done* and close the milestone task;
* **wrong, and how**: a sentence and, if you can, where (map, vehicle, position). I record it as a defect under the milestone (the [intake rule](../../CONTRIBUTING.md)) and fix it;
* **not now**: nothing changes.

Keys used below, all from the code or the camera spec: **1** ego, **2** driver's view, **3** external, **4** passenger, **5** free roam, **0** previous view, **W** throttle, **arrow keys** steer (A and D belong to the gear selector), **Shift + Space** resets the current view, **Shift + Tab** opens the viewpoint editor, **Tab** the virtual mirrors, **F1** the in-game Help with the full list of controls. Commands starting with `opts-cli` need a running game with the control server on.

## A. The Review column (waiting since 2026-10-11 and later)

| # | Item | Where and how | What you should see |
| ---: | :--- | :--- | :--- |
| 1 | **Articulated bus** ([M-0022 addendum](../src/architecture/articulation.md), [M-0053](../src/roadmap/phase-4/M-0053.md)) | Spawn an articulated bus (any with a bellows, e.g. a MAN NG). Press **3**, drive a right-hand curve, then a hill. | The rear unit follows the front one and lags on a right-hand curve; the joint stays closed; the bellows folds follow the two halves without gaps. Open question this answers: do the signs of the joint angles match OMSI's (Q-ART-01, Q-ART-02)? |
| 2 | **Repaints** ([M-0032](../src/roadmap/phase-6/M-0032.md)) | In *Plan a drive*, step ② *What*, pick a vehicle with several repaints, choose one other than the default, start. Also try one AI or parked vehicle with a repaint. | The chosen repaint's textures on the vehicle; AI and parked vehicles show repaints too. Compare with [repaints.md](../src/formats/text/repaints.md) (Q-RP01, Q-RP02). |
| 3 | **DXT mip chains** ([M-0058](../src/roadmap/phase-8/M-0058.md)) | Drive on a map with long straight roads and detailed ground textures (Köln). Watch the far road and building textures at speed. The first load of a map takes about 4 s longer; a second load about 0.2 s (cache in `~/.cache/opts/textures/`). | No shimmering of distant textures while driving. This is the open question: does the far view look steady? |
| 4 | **Driver's view from the `.bus` preset** ([M-0084 camera](../src/architecture/camera-system.md)) | Press **2**: the eye should sit at the vehicle's driver camera, not at the character's head. Press **1** and **2**: the driver figure should be hidden in both. Open the Alt menu, *Jump to entry point*. | Eye at the preset position; no driver body in your view; the jump puts you in a free-flight view of the entry point. |
| 5 | **Invisible walls** ([M-0090 addendum](../src/simulation/collisions.md)) | Köln, Frankenstraße and the other places where you met them. Drive along the street, over junction plates, past street lamps. | No stopping at street and junction plates; a lamp is not a wall 3.7 by 7.5 m wide. |
| 6 | **Mesh fidelity** ([M-0066](../src/roadmap/phase-10/M-0066.md)) | Maps with parked cars (Trabants, MB T1, Peugeots), the BRT Citaro, buildings that looked flipped, lantern posts. | Parked cars present and intact; the Citaro and the buildings as authored; lantern posts not scrambled. The board says R-09, R-17 and R-18 were re-checked without finding a defect: say if you still see one. |
| 7 | **Roadmap review dry run** ([M-0119](../src/roadmap/phase-11/M-0119.md)) | Not in the game: read the [report](../src/meta/roadmap-review-2026-10-20.md). | Whether the review format is useful, and what to change in the prompt. |

## B. Changes since 2026-10-26 that have only been checked headlessly

| # | Change | Where and how | What you should see |
| ---: | :--- | :--- | :--- |
| 8 | **Pedestrians keep to the right** (M-0050, 2026-10-29) | A map with pedestrians (*Settings*, pedestrian density above zero). Press **5** and watch a footpath where people walk both ways. | Everyone walks beside the centre of the path, on the right-hand side of their direction; people coming towards each other pass on different sides. They still walk through each other when going the same way at the same spot. |
| 9 | **Smooth driver view** (M-0109, 2026-10-29) | Press **2**, drive at 60 to 100 km/h through some curves. | A steady view without stutter. A headless drive measured 1.9 mm of deviation per frame; this checks that a real display shows nothing the headless run could not. |
| 10 | **Passenger weight** (M-0124) | `opts-cli vehicle.passengers mass_count=60`, accelerate from rest and brake from 50 km/h; then `opts-cli vehicle.passengers mass_count=-1` and compare. | With 60 people the bus accelerates and brakes noticeably more slowly and sits lower on its springs. `mass_count=-1` goes back to the real passengers. |
| 11 | **Live mirrors** (M-0085, 2026-10-28) | Drive in a town with parked and AI vehicles. Look at the mirrors of other vehicles and at your own. | Every mirror in view shows a live picture, not a fixed scene. Frame rate stays acceptable. |
| 12 | **Seated passengers' legs** (M-0101, 2026-10-28) | Board passengers at a stop (or `opts-cli vehicle.passengers spawn=20`); press **4** or look from inside. | Seated passengers' legs reach the floor and fit the seat. |
| 13 | **Dashboard hot spots** (M-0141, 2026-10-28) | Click dashboard buttons whose model is built from nested parts. | The click reaches the control (the child meshes answer the mouse event). |
| 14 | **Bellows under an animation parent** (M-0053, 2026-10-28) | An articulated bus of the Valiant or Heuliez kind. Drive a curve and look at the rear half of the bellows. | The rear half of the bellows follows the trailer fully, not by half the angle. |
| 15 | **Viewpoint export and import** (M-0084, 2026-10-29) | Set a viewpoint (**Shift + Tab**), then `opts-cli camera.viewpoint.export ~/.local/share/opts/reports/viewpoints.json`; change it; `opts-cli camera.viewpoint.import ~/.local/share/opts/reports/viewpoints.json`. | The saved viewpoint comes back. (Protocol only: no menu entry exists. Say if you want one.) |

## C. What this page does not cover

* Anything that is a decision rather than a look, such as red-light running or what `gameplay.time_scale` should mean; those are in the [gap analysis](../src/meta/gap-analysis-2026-10-29.md) and the milestone files.
