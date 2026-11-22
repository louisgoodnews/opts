# ISS-054: Roaming emergency and service vehicles depend on the map's global.cfg

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-054. Owner request 2026-11-22 (new feature; not a research item) |
| **Severity** | minor (provisional; the groups already spawn, so the gap is the setting and the siren rule) |
| **Milestone** | [M-0173](../../src/roadmap/phase-18/M-0173.md) Roaming Emergency and Service Vehicles (global.cfg) (proposed 2026-11-22); collision response with police and ambulance in [M-0171](../../src/roadmap/phase-18/M-0171.md) |
| **Status** | milestone M-0173 proposed (owner request 2026-11-22; not approved to start) |

## Problem

The owner asks for emergency and other roaming services (police, ambulance and similar) that are active on the road. Their presence should depend on the map's `global.cfg` roaming setting.

- The map format page ([global.cfg](../../src/formats/map/global-cfg.md)) lists the traffic curves `[trafficdensity_road]` and `[trafficdensity_passenger]`. It lists no roaming or service key. The setting the owner describes is not documented there today.
- The groups themselves are in the map's AI lists: `ailists.cfg` defines each group, and `unsched_vehgroups.txt` names the unscheduled ones. The lists page shows `Ambulance` and `Emergency` as group names ([lists](../../src/formats/map/lists.md), `[aigroup_2]`).
- The AI traffic page says that every group a map lists now drives, including `Polizei`, `RTW` and `Feuerwehr` (2026-10-26, [ai-traffic](../../src/architecture/ai-traffic.md), line 217). An earlier census said that only `NormalCars` was used ([ai-vehicle-config](../../src/meta/ai-vehicle-config-2026-10-14.md), line 135); that is superseded by the 2026-10-26 change.
- Emergency vehicles read `TrafficPriority` and `TrafficPriorityWarningNeeded` for their siren logic ([ai-traffic](../../src/architecture/ai-traffic.md), line 56). M-0049 still lists "`TrafficPriority` for emergency vehicles" as open.

So the groups already spawn. What is missing is the map-level setting (its source is not yet established), the siren priority, and a check of the behaviour near the player.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). The format is described from OPTS's own parser and the public format pages only. Scripts may be read in prose for logic and variable names. No OMSI script text or sound text is copied into the repository or into this file.

## Work

1. Read [global.cfg](../../src/formats/map/global-cfg.md) and [lists](../../src/formats/map/lists.md) again. If a roaming or service key exists in the corpus, document it as a format fact with its counts. If none exists, record that the setting the owner describes is not in the format, and name the file that carries the group instead.
2. Confirm on one map that the emergency groups spawn and drive, and record which ones.
3. Decide the siren priority with M-0049 and M-0111: either here, or in M-0111 with a reason.
4. Check the density of these groups against the map's traffic curve, as for the other AI groups.

## Done when

- The format page states, in prose, whether `global.cfg` controls roaming or service vehicles, and which file does if not.
- One map is checked, and the result is recorded in M-0173.
- The siren priority is decided, with the milestone that holds it.
