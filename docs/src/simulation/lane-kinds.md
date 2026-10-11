# Lane Kinds and Composite Splines

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Classification implemented (2026-10-11); bike traffic itself not yet |
| **Code** | `opts_core::formats::scenery::{LaneKind, SplinePath::kind, SplineProfile::{lane_kinds, composition, has_bike_lane}}`, `opts_sim::pedestrian::lanes_of_kind` |

## Why

OMSI's `[path]` has four types (`0` road, `1` pedestrian, `2` rail, `3` aircraft). One profile usually carries several: a street with sidewalks has road lanes **and** footpaths, a tram street has road **and** rail lanes. Each kind is a **sub-spline** with its own traffic, rules and simulation, so OPTS classifies per path instead of per spline. Bikes have no type of their own in OMSI.

## Rules

| Kind | Rule |
| :--- | :--- |
| Road | type `0`, width > 0.6 m (and not bike-named, see below) |
| **Bike** | type `0` with width ≤ **0.6 m**, or type `0` with width < 1.5 m in a profile whose file name contains `fahrrad`, `bike`, `radweg` or `cycle` |
| Pedestrian | type `1` |
| Rail | type `2` |
| Aircraft | type `3` |

`SplineProfile::composition(name)` lists the distinct kinds of a profile (a composite has more than one); `has_bike_lane(name)` answers the bike question.

## Evidence (corpus, 12,510 road-type paths)

* `Invis_Human_Fahrrad.sli` (the helper profile for cyclist paths) holds one footpath (type 1, width 0.1, bidirectional) and **two road-type paths of width 0.1 at ±0.85 m**: the bike lanes.
* Road-type paths by width: 0.1 m ×59, 0.2 ×8, 0.3 ×4, 0.5 ×45, **1.0 ×1,118** (e.g. `ÖPNV_BOT.sli`, a bus-only guideway), 1.2 ×93, 2.0 ×1,125, 2.5 ×1,319, 3.0 ×4,800, 3.5 ×3,560. The threshold 0.6 m separates the narrow helper lanes from every motor-vehicle width; 1.0 m guideways stay road lanes.
* 115 profile files have bike names. Pure marking profiles (`bikeline*.sli`, `bike_pik*.sli`, `rad_u.sli`) have **no paths**: they paint, they carry nothing. Combined profiles such as `RQ_9,5_2spur_6,5m_sidewalk+fahrrad.sli` have a type-1 path of width 0.98 at ±6.25 m named for bikes: in these the *bike lane is a pedestrian-type path* ❓, so a name hint on a type-1 path of width ≥ 0.9 also marks a shared foot/bike way. (Not yet classified as `Bike`: the rule above leaves it a `Pedestrian`; shared use is an open modelling question.)
* Inferred, not documented by OMSI ❓: no source states that narrow road paths mean bikes.

## Using it

`lanes_of_kind(inst, profile, name, shift, LaneKind::Bike)` samples the bike lanes of a placed spline like footpaths (same arc and height rules), so a rider can use the pedestrian network machinery (`PedestrianNet`, lane ends joined within 0.6 m) with a different walker: speed 4–7 m/s, no turning round on one-way lanes, priority at crossings. A cyclist agent and the interaction with road traffic belong to the AI-traffic work ([ai-traffic](../architecture/ai-traffic.md)); the hook is `LaneKind::Bike`.

## Open

* Shared foot/bike ways (type 1, bike-named) and cycle tracks drawn only as markings (no path): riders would need a path derived from the marking profile's position.
* Maps with real bike traffic: the Hamburg maps were checked by profile name only; none of the installed Hamburg tiles uses a bike-named *spline* (counted 0 in `Hamburg109`), so bike paths there come from `Invis_*` helper objects or the `.sco` paths ([`sco.md`](../formats/text/sco.md#paths)) — to be surveyed.
