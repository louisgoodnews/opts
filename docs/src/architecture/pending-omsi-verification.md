# Pending OMSI Verification

| Metadata | Value |
| :--- | :--- |
| **Status** | Living list (2026-10-14) |
| **Purpose** | Everything the project decided or assumed that only a running OMSI can confirm, in one place, so that none of it is deferred silently |

Each row names the assumption, where it is built in, what changes if it is wrong, and which protocol settles it. When an item is answered, move it to the bottom table with the date and the person who measured it.

## Pending

| ID | Assumption in the code or docs | Built into | If wrong | Settled by |
| :--- | :--- | :--- | :--- | :--- |
| **Q-AIC-002** | `[rowdy_factor]` (539 vehicle files; a per-vehicle draw between two numbers) scales the desired speed by ±10 %: `0 0` drives at 90 % of the open-road speed. **Visible effect today: the median desired speed on Aachen Linie 33 is 45 km/h instead of 50 km/h** | `IdmParams::from_traits`, `Calibration::speed_spread` | traffic on maps of calm vehicles is about 10 % too slow (or the field scales something else, such as headway or acceleration) | A7 of the [AI calibration protocol](ai-calibration-protocol.md) |
| Q-AI01 | free-road speed 50 km/h (literature), headway 1.5 s / 1.7 s, minimum gap 2 m, comfort fractions 0.55 (drive) and 0.15 (brake) | `Calibration::default()` | AI speed, spacing and braking differ from OMSI's | A1-A6 of the AI protocol |
| Q-AI04 | one vehicle per 250 m of road at density factor 1 | `METRES_OF_ROAD_PER_VEHICLE` | traffic too thin or too dense | A9 of the AI protocol |
| Q-AIC-001 | units and role of `AI_M_gas_max`, `AI_v_max`, `AI_v_max_erreich` (inferred from the shared script law) | `AiProfile`, `from_traits` | per-vehicle acceleration off by a constant factor | A3, A4 |
| Q-AIC-003 | only the first number of `[ai_brakeperformance]` matters, as a multiplier | `AiProfile::brake_performance` | wrong braking for the 141 vehicles with the keyword | A8 |
| Q-AIC-005 | `[type] 2` without `[boogies]` is a road vehicle (two scooters) | `AiProfile::domain` | scooters wrongly kept or dropped | inspect in OMSI |
| Q-BSI-1 | unit of `M_Wheel` is N·m, summed over driven axles | script drive (`--script-drive`), strategy P1 | every bus too strong or weak | E1/E1b of the [OMSI experiment protocol](omsi-experiment-protocol.md) |
| Q-BSI-3 | `STTextOut` arguments 5 and 6, `STSetColor` order and alpha, `STLoadTex` semantics, `GetHeightAbovePoint` | callbacks reference (inferred) | displays differ in spacing or colour | E2-E5 |
| Q-BSI-4 | which triggers and events the engine raises itself | strategy P6, P5 | missing events | E7 (deferred) |
| Q-BSI-5 | a trigger defined in several parts reaches all of them | strategy P2 | doors of the wrong part react | E8 |
| Q-BSI-6 | passenger index space of `GetHumanCountOn*` | strategy P5 | wrong counts on articulated buses | E6 |
| Q-VS-3 | OMSI's physics step is fixed or variable; scripts integrate with `Timegap` | OPTS runs a fixed step | integrated script quantities differ | rig runs at 30/60/120 Hz, then a comparison with OMSI |
| R1-R6 | hof column to display mapping, route code semantics, timetable and delay sign, neutral values without a timetable | strategy P3 | wrong strings on displays | session 1 of the experiment protocol |

## Answered

_None yet._
