# AI Traffic Calibration Protocol (experiments on a running OMSI)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Protocol (2026-10-14); no measurement has been taken |
| **Settles** | Q-AI01, Q-AI04 of [AI traffic](ai-traffic.md); Q-AIC-001 to Q-AIC-003 of [AI vehicle configuration](ai-vehicle-config.md) |
| **Feeds** | the constants of `opts_traffic::Calibration` (`open_road_speed`, `speed_spread`, `accel_fraction`, `comfort_brake_fraction`, `headway`, `min_gap`) |
| **Needs** | a running OMSI 2 with a test map and a copy of one AI vehicle folder; about half a day |

The traffic model maps declared vehicle data to driving numbers with clamped formulas; the *behaviour constants* in `Calibration` are literature values until OMSI is measured. This protocol says what to record so the constants can be set from evidence. Nothing here inspects OMSI's code: all measurements are taken by driving and watching, in the spirit of the [clean-room policy](../adr/0002-clean-room-policy.md).

## Setup

* A map with a straight, flat road of at least 1 km and a signalised junction (Waldhofen or Grundorf), traffic on, density default.
* Record the player vehicle's `Velocity` readout (km/h) and a stopwatch or video at 30 fps. A player bus following an AI vehicle at the same speed gives that vehicle's speed.
* Never edit the installation: for E5 copy one AI vehicle folder under a new name and add it to a *copy* of the map's `ailists.cfg`.

## Experiments

| ID | What to do | Record | Derives |
| :--- | :--- | :--- | :--- |
| A1 | Follow ten different `NormalCars` vehicles on a road with no `speedlimit` rule at constant gap; read the speed | vehicle file, speed (km/h) | `open_road_speed` (median) and spread; whether it depends on `AI_v_max` |
| A2 | The same on roads with a 30, 50 and 70 km/h limit | limit, speed | whether the speed is the limit, a fraction of it, or capped |
| A3 | Watch an AI vehicle accelerate from a red light; film the speedometer of a bus beside it | time to 20 and 40 km/h, vehicle | `accel_fraction` per vehicle class; compare with `AI_M_gas_max / (r · mass)` |
| A4 | Same for a lorry and a bus from the AI lists | same | class dependence; the mass term |
| A5 | Follow an AI car at constant speed and measure the gap in metres (a marker on the road or the bus bumper against a known object) and the speed | gap, speed | time headway `T` = gap / speed; minimum gap `s0` from a standing queue |
| A6 | Observe AI vehicles stopping for a red light from about 50 km/h: distance from the stop line where braking starts, and the stopping distance | distances, vehicle | `comfort_brake_fraction`, comfortable deceleration |
| A7 | Copy one AI car folder; set `[rowdy_factor]` to `0 0` in one copy and `1 1` in another; add both to a test list with weight 1 | speed, headway, acceleration of each | what `[rowdy_factor]` scales (Q-AIC-002) |
| A8 | Same with `[ai_brakeperformance]` `0.5` and `1.5` as first number | stopping distance | the brake multiplier (Q-AIC-003) |
| A9 | Count the AI vehicles within 300 m of the player on a known road length at density factors 0.1, 1 and 2 (`unsched_trafficdens.txt` copy) | counts, road length | `METRES_OF_ROAD_PER_VEHICLE`, Q-AI04 |

## Recording format

One CSV per experiment: `experiment,vehicle,limit_kmh,value,unit,notes`, for example `A1,ai_hh109kipkw1.bus,,48,km/h,`. Keep the video where possible: a later reader can re-measure.

## From measurements to constants

* `open_road_speed` = median of A1 divided by the mean speed factor of the sampled vehicles' rowdy ranges (the model multiplies by `1 + speed_spread · (2·rowdy − 1)`); `speed_spread` from the spread of A1 (half the relative range between the 10th and 90th percentile).
* `accel_fraction` = measured mean acceleration between 0 and 20 km/h (A3, A4) divided by `AI_M_gas_max / (wheel radius · mass)` of the same vehicles; fit per class and keep one value if the classes agree within 20 %.
* `comfort_brake_fraction` = measured comfortable deceleration (A6: `v² / (2 · braking distance)`) divided by `AI_F_brems_max / mass`.
* `headway` = median `T` from A5 for cars and for buses/lorries; `min_gap` from the standing queue.
* Each constant gets an interval, not only a value; the calibration test in `opts-traffic` then asserts that the model's output for the measured vehicles falls inside the intervals.

## Where the numbers go

Update `Calibration::default()` and the values quoted in [AI vehicle configuration](ai-vehicle-config.md) (design, section D2), mark Q-AI01, Q-AI04 and Q-AIC-001 to 003 answered with the date and the measuring person, and keep the CSV files under `docs/src/meta/data/` (small text files only).

**Clean-room attestation (ADR-0002):** observation of the running product only; no binaries inspected.
