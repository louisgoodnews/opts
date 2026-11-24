# On-screen check: F12 refuel refusal toast (M-0123)

Date: 2026-11-24. Master at the time: 26980ff6. Build: debug opts-game from the worktree, windowed
on the Wayland session, scene Bad_Huegelsdorf_2020 with MAN_SD200/MAN_SD77.bus, ego camera.
Speed and fuel were read through the control protocol (drive.state, vehicle.fuel). F12 was sent
through the input layer (ui.key F12); screenshots through the protocol's screenshot command.

| Check | Result | Evidence |
| :--- | :--- | :--- |
| Refusal while moving (1.54 m/s and 1.90 m/s) | PASS: fuel unchanged at 50 %, two toasts "Refuel refused: stop the vehicle first (below 0.5 m/s)" | sim 59.2 s and 62.3 s; fuel 125.0 L, 124.93 L, 124.89 L (consumption only) |
| Refuel while stopped (0.0 m/s) | PASS: fuel 124.85 L to 249.999 L (100 %), no toast | sim 82.2 s, brake applied |

Screenshots are in the project scratch folder and are not committed (they show game content).
The threshold boundary (0.49 and 0.51 m/s) is not checked on screen; the tests cover it.

Notes from the run: the debug opts-cli binary in the scratch target predates this build (it
worked for every command used). The disk for the scratch target was full during the run.
