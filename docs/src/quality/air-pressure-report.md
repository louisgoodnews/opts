# Air Pressure System Report (M-0141)

*State of the pneumatic system of the player's vehicle, checked live on 2026-10-20 on the MAN New Lion's City 18C (NLC) with the probe `air-probe.py` (the scratch harness of the session; every number below is a reading of `vehicle.script_vars` and `vehicle.dynamics`). Other vehicles were not measured; the read of their scripts is in [script integration report](../architecture/script-integration-report.md) and [vehicle functions](../simulation/vehicle-functions.md).*

## How it is implemented

The air system is **entirely the vehicle's own script**: tanks, the compressor and its governor, valves, brake cylinders, bellows and the consumers (doors, kneeling, brakes) are variables the `.osc` files advance each frame (`bremse*.osc`, `door-3.osc`, `bremse_Gelenkzug21.osc` for the NLC). The host does three things:

1. runs the scripts and feeds their inputs (`Brake`, `Velocity_Ground`, `Axle_Suspension_*`, `Wheel_RotationSpeed_*`, `Timegap`, `random` for the start pressure);
2. takes the **brake force the script writes** (`Axle_Brakeforce_<n>_<l|r>`, newtons; the total `Brakeforce` for AI scripts) and hands it to the wheel physics (`vehicle_script.rs`); with a script that writes the forces the host's own `BrakeSystem` (`opts-sim/src/vehicle/brakes.rs`, pneumatic build-up, rated-load sizing) is idle;
3. shows the variables where the model reads them: the gauges (`bremse_p_tank*` through `model.cfg`), and the sound events the scripts raise (`ev_bremse_*`, `ev_kompressor_*`, `ev_loesezischen`).

There is no air system in the host: no tank, no compressor, no pressure is simulated outside the scripts. A vehicle without a brake script falls back to the host's brake model.

## What was measured (NLC)

| Check | Reading | Verdict |
| :--- | :--- | :--- |
| Initial pressure | tanks 7.05 to 7.09 bar after load (`random` start pressure of the script) | works |
| Compressor | fills the four circuits at about 0.023 bar/s, 7.1 → 9.49 bar in about 100 s; governor cuts out at **9.49 bar** (`bremse_kompressor` 1 → 0) | works |
| Compressor and the engine | the script's flow is `engine_n · Timegap · m_compressor` (`bremse_Gelenkzug21.osc:368`), governor cut-in 8.0 and cut-out 9.5 bar; the test vehicle was already running from load (`vehicle.setup` applies at spawn), so the engine-off case was **not** measured (an earlier reading of this page called it a fault: that was wrong) | works; engine-off not measured |
| Brake application | full pedal: cylinder pressure follows the tank (8.45 bar at 8.44 bar tank), force 24.9 kN per wheel side on the front axle, 100 kN in all (as `max_bremskraft` says) | works |
| Air consumption | three full applications took the first circuit from 9.47 to 7.5 bar (about 1 bar each); the compressor recharges | works |
| Doors | one open and close cycle took tanks 3 and 4 from 8.20 to 8.04 bar (`door-3.osc` consumes from tank 4) | works |
| Kneeling | needs more than 5.5 bar in tank 3 (`kneel_allowed`), took 745 → 724 kPa while kneeling; the script lowers `Axle_Springfactor_<n>_R` to 0.55 (the NLC kneels on the kerb side only; `kneel_timer` 5 says the depth is reached) | the script works; the host did **not** lower the body, **fixed** (below) |
| Host view | `vehicle.dynamics` `brakes` reads `{force_n: 0, pressure: 0}` while the script brakes with 100 kN | misleading: the field is the host model's, idle here |

## Fixed on the way: kneeling and raising

The host multiplied only the *deflection from rest* by `Axle_Springfactor`, so a factor never changed the ride height, only the stiffness around it: the NLC kneeled in the script and stayed at its height in the game. The factor now scales the whole spring, the rest load included (`suspension.rs`: `k · f · c + (f − 1) · static_load`), so 0.55 settles the body lower (about `(1 − f)/f · static/k`, 7 to 8 cm for the NLC's front axle) and a factor of 2 (raising) lifts it. Test `a_softer_spring_lowers_the_body_and_a_stiffer_one_raises_it`.

## Open

* **Low pressure behaviour** (the spring brake applying below about 4 bar, the warning buzzer, no brake force at an empty tank) is the script's; it was not provoked here.
* **`vehicle.dynamics` should show the script's brake state** (cylinder pressures and forces) when the scripts drive the brakes: its `brakes` field is the idle host model's.
* **The compressor with the engine off** and the gauges were not looked at in this run.
