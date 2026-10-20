# Passengers

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 First slice 2026-10-21: stops, boarding through the scripts' doors, ticket decisions, places, alighting, the counts the scripts read |
| **Code** | `opts_sim::passengers` (cabin graph, passengers, tickets), `opts_omsi::formats::{ticketpack, vehicles::cabin}`, `opts_bevy::bus_stops`, `opts-game` `passengers.rs` and `omsi_world/vehicle/passengers.rs` |
| **Milestone** | [M-0101](../roadmap/phase-4/M-0101.md) |
| **Specs used** | [ticket packs](../formats/text/ticketpacks.md), [`passengercabin.cfg`](../formats/text/vehicles/passengercabin_cfg.md), [`paths.cfg`](../formats/text/vehicles/paths_cfg.md), [money](../formats/text/money.md), [`.sco`](../formats/text/sco.md) (`[busstop]`), [pedestrians](pedestrians.md) |

## 1. What happens

A bus that stands at a bus stop gets passengers: people appear beside the stop, walk to the nearest entry door, wait until the vehicle's own scripts report the door open, step in, decide about a ticket, take a seat or a standing place, ride, and when their ride is over walk to the nearest exit, wait for it to open, get off and walk away until they vanish.

* **Stops.** An object whose `.sco` carries `[busstop]` is a stop (the generic `Sceneryobjects\Generic\bus_stop.sco` is such a marker, hidden in the game; 26 near the start of Ahlheim 5). Its position and heading are where the bus stops; the kerb is on its right (`opts_bevy::bus_stops::BusStopMarker`). The bus is *at* the stop when it is slower than 3 km/h within 22 m of the marker, and has *left* when faster than 6 km/h or 45 m away. `Busstops.cfg` is not needed (the Ahlheim files are empty); names and timetables stay with [M-0099](../roadmap/phase-4/M-0099.md).
* **Waiting people.** 0 to 7 per visit (a modelling choice, not an OMSI number), characters drawn from the map's `humans.txt` like the pedestrians ([`PedestrianPool`](pedestrians.md)), standing beside the marker and walking to the entry nearest to them at 1.3 m/s.
* **Doors are the scripts'.** The scripts write `PAX_Entry<n>_Open` and `PAX_Exit<n>_Open` for the n-th `[entry]` and `[exit]` of the cabin (the NLC's first door opens entries 0 and 1, its second entries 2 and 3 and exits 0 and 1). Nobody enters a closed door; a person waiting at a door raises `PAX_Entry<n>_Req`, a person waiting to get off `PAX_Exit<n>_Req`, and the scripts see `humans_count` (all `PassengerFeed` on the vehicle). A script that opens its doors by itself on a request (the NLC does for `Door01_req` and similar) works through this.
* **Tickets.** On entering, a passenger buys with the chance `ticketbuy_prop` of the map's `.otp` (`[ticketpack]`; a ticket drawn by `probability` among those the age and the number of stops allow), walks to the first `[ticket_sale]`, pays (4 s, the price joins `revenue`) and is stamped; otherwise they stamp an existing ticket with `stamper_prop` at the first `[stamper]` (1.5 s). A cabin with `{noticketsale}` (the NLC) sells nothing: those passengers only stamp. If the map names a pack that is not installed (Ahlheim 5 names `VRR 2020_AL`), a built-in pack of four tickets is used and a warning logged.
* **Places.** A seat with a chance of 70 % when one is free, else standing; the nearest by walking distance on the cabin's path graph (`paths.cfg`, links walkable both ways). A full cabin leaves the rest standing in the aisle.
* **Riding.** Each passenger rides 1 to 7 stops; the bus reaching a stop counts one for everybody inside; at zero they head for the nearest exit.

## 2. What the scripts read

| Host value | Source |
| :--- | :--- |
| `humans_count` | passengers inside |
| `PAX_Entry<n>_Req`, `PAX_Exit<n>_Req` | somebody waits at the door (entries also while people stand at the door) |
| `GetTicketName(i)`, `GetTicketValue(i)` | the map's pack (the name with `@` as a line break, the price) |
| `GiveChangeCoin(i)` | coin `i` of the map's money system is logged (`change_given`); the cash desk that uses it is open |
| `GetHumanCountOnSeat`, `GetHumanCountOnPathLink` | **still 0**: the simulation has the counts (`PassengerSim::on_place`, `on_link`) but the callbacks do not read it yet |

## 3. Verified

* Unit tests: the `.otp` parser (64 installed packs, 716 tickets), the cabin parser (121 installed cabins, 4,376 places), the cabin graph, a passenger boarding only through an open door, buying, paying, taking a place, a full cabin, alighting at a stop, link counts, determinism (`opts-sim/src/passengers/tests.rs`).
* Live on the NLC, Ahlheim 5, with `vehicle.passengers`: at a real stop two people appeared, waited for the door, boarded when it opened and took places; ten people sent to the door boarded in under five seconds, two stamped (none bought: `{noticketsale}`), rode three stops, waited at the exit until the second door opened and all ten got off, walked away and vanished.

## 4. Open

| Topic | Note |
| :--- | :--- |
| Callbacks | `GetHumanCountOnSeat` and `GetHumanCountOnPathLink` read the simulation (the index space is E6 of the [script integration strategy](../architecture/bus-script-integration.md)). |
| Cash desk | The driver's side of the ticket sale: choosing the ticket, taking money, giving change (`GiveChangeCoin`, `ticket_give`, `change_give`). Passengers pay by themselves for now. |
| Articulated buses and trams | Only the main unit has a rig; the following units' cabins and the `[linkToPrevVeh]` links are not chained. |
| Alighting at *real* next stops | The ride counts stops reached; the stop sequence of the timetable (`bus.set_stop`) and the destination of each passenger are not coupled. |
| Seated pose | The place and rotation come from `[passpos]`; the height of the seated body against the seat was not tuned against the models. |
| Crowds | No collision between people; the number waiting is uniform, not weighted by the time of day (`[trafficdensity_passenger]`). |
| Voices | Greetings, requests and complaints (`TicketPacks/*/<voice>/`) are not played. |
| Setting | There is no off switch yet. |
