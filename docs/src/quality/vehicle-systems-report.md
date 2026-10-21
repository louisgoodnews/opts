# Vehicle Systems Report (M-0141)

*What the vehicles' own scripts do in OPTS, system by system, checked live on 2026-10-20 and 2026-10-21 (mostly the MAN New Lion's City 18C, the Bremen MAN NG313 and the Tram NF6D; probes are `vehicle.script_vars`, `vehicle.click`, `vehicle.key`, `vehicle.dynamics`, `vehicle.passengers` and screenshots). Detail pages: [air pressure](air-pressure-report.md), [start procedures](vehicle-start-procedures.md), [controls](controls-report.md), [passengers](../simulation/passengers.md).*

The principle: the `.osc` scripts run unchanged; the host feeds their inputs (`opts_bevy::vehicle_script`), takes what they write for the physics (torque, brake force, spring factors) and shows the rest through the model's animations and the sound events. A "system" works when its script runs *and* what it writes has an effect.

| System | Script runs | Effect in the game | Verified |
| :--- | :--- | :--- | :--- |
| Electrics, ignition, engine start | yes | the battery key, the ignition and the starter bring the engine up from cold ([procedures](vehicle-start-procedures.md)) | NLC, NG313, Citybus, tram |
| Engine, gearbox | yes | `M_Wheel` drives the wheels; the automatic shifts | NLC, NG313, NG272, Urbino (does not move in the harness, open) |
| Service brake and parking brake | yes | per-wheel brake force from the script's own pressure model; 100 kN in all at full pedal on the NLC | NLC |
| Air pressure | yes | tanks, compressor with governor (cut-in 8.0, cut-out 9.5 bar), consumers: doors, kneeling, brakes ([report](air-pressure-report.md)) | NLC |
| Kneeling and raising | yes | **fixed 2026-10-20**: the script's spring factor now scales the whole spring, the NLC kneels on the kerb side (3.8 cm lower, 1.7° of roll) and rises again | NLC |
| Doors | yes | the leaves animate by door type (interior swing, outward swing, outward-swinging sliding; corrected 2026-10-20 in [animation axes](../architecture/animation-axes.md)), `PAX_*_Open` goes to the passengers, door sounds | NLC (all three types), NG313 |
| Door keys | yes | per vehicle kind: buses Num / and Num * (doors 1, 2), Ctrl+Num / and Ctrl+Num * (doors 3, 4), Num - clearance, Num + all doors (the NLC has no stadium switch); trams clearance and side toggle ([keys](../simulation/vehicle-keys.md)) | NLC, NF6D |
| Lights, cabin lighting | yes | headlights, indicators, interior groups on 6 to 9, the cabin light on Ctrl+L (the cab lit at night) | NLC, tram |
| Heating | yes | the controls turn (horizontal drag for the temperature and fan knobs), the fan runs up to 100 rpm, the cabin air warms from 17.8 to 18.9 °C in 80 s with the engine running | NLC |
| Wipers, mirrors, sun blind | yes | the wiper animates; the NG313's blind was fixed 2026-10-20 (translation sign of mesh-origin animations); the blinds of other vehicles are not all checked | NG313 |
| Sound | yes | engine loops by rpm and load, rooms inside and outside, door and switch one-shots, trailer units' loops ([audio](../architecture/audio-system.md)) | five vehicles |
| Passengers, tickets | yes | people at stops board through the doors the scripts open, stamp or buy, take places, get off; `humans_count`, `GetHumanCountOn*` and the ticket callbacks read the simulation and the map's ticket pack | NLC, NG313 |
| Ticket printer (`Drucker`) | yes | a script-drawn dialog on a display texture (login, menu, ticket choice): the ticket names now come from the map's ticket pack (they were one fixed text) | script only; the display texture was not looked at |
| Cash desk (change dispenser buttons) | yes | `GiveChangeCoin` logs the coin of the map's currency; no coin is drawn at the change point yet | log only |

## Not working or not checked

* **Driver displays** (IBIS, matrix, printer, dashboard screens): the script textures belong to [M-0100](../roadmap/phase-4/M-0100.md).
* **Other vehicles** than the five above: only the controls census ([report](controls-report.md)) covers them.
* **Cash desk visuals**: coins at the change point, money from passengers on the desk, the driver's choice of ticket for a waiting passenger ([passengers](../simulation/passengers.md), open).
* **Trams**: the NF6D runs its scripts; its passengers are not set up.
* **Urbino 12** does not move in the sound harness (gear selector and trigger mismatch, not investigated).
