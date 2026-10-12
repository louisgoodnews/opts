# Collisions of the Player's Vehicle

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 First version (2026-10-12): static objects, AI vehicles, ground steps for road vehicles |
| **Code** | `opts_sim::collision` (boxes, response, grid), `opts-game` `in_game/omsi_world/collision.rs` |

## Model

* **Boxes.** Everything collides as an oriented box on the ground plane with a height range (`Obb`): separating-axis test on the edge normals gives the shortest push (`Contact`).
* **Player.** The head unit's box is the `.bus` `[boundingbox]` (full extents and centre offset, x lateral, y longitudinal; its height starts at the wheels). Each following part of a chain contributes its own `.bus` box at its simulated pose. Rail vehicles are ghosts, as in OMSI (Tram SDK: collisions of player rail vehicles are always off; buses can collide with trams).
* **Scenery.** Every placed `.sco` with `[collision_mesh]` (and without `[nocollision]`) registers the bounding box of each collision mesh (`CollisionIndex`, a 16 m grid; the tile's boxes leave with the tile). Protected (scrambled) collision meshes are skipped; trees and editor-only objects have none. A map tile of Hohenkirchen registers about 1,200 boxes.
* **AI vehicles.** Their box is measured from their meshes' bounds (shadow volumes excluded) once loaded and follows the vehicle; the player is pushed out (the AI agent is kinematic and does not react yet).
* **Ground steps.** Eight points on the head's outline are compared with the ground 0.4 m closer to the centre; a rise above 0.3 m blocks (walls, embankments, platform edges). Falling down a step is allowed and the ground follower lowers the vehicle.
* **Heights.** A box whose top is within 0.3 m of the vehicle's wheel level (kerbs, road plates) is driven over; one that starts above the vehicle's height is passed under.

## Response

Fixed step after the physics, before the articulation: up to four passes push the chassis out of the deepest overlap (at most 0.5 m per step), then the velocity loses its component into the obstacle (bounce 0.05), 8 % of the sliding speed is scraped off and the yaw rate is damped. Checked on Hohenkirchen: a bus driven into a shelter at 26 km/h stops within the step.

## Not yet

Damage and sound (`coll_energy`, `collision` trigger of the scripts: the data is there, the events are not raised), pedestrians, AI reaction to the player (the traffic agents do not see the player), trailer pivots reacting to the push (the push moves the head only), per-wheel curb climbing, tree trunks, `[crashmode_pole]`, collision meshes of protected objects.
