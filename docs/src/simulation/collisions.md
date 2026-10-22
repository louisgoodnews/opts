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

## Implementation strategy for the open features

Ordered by value over effort; each step is one commit with a test on the pure `opts_sim::collision` layer where possible.

1. **AI traffic sees the player.** Register the player's `PlayerBox` (all parts of the chain) as a dynamic obstacle in the traffic crate's neighbour query (`opts-traffic`: the agents already look ahead for other agents; add a "foreign body" list filled each fixed step from `CollisionIndex`/`PlayerBox`). Agents then brake for the player like for a leading agent (distance along their lane within the lookahead, lateral overlap with the lane corridor). Player contact does not push agents; a hit stops the agent (kinematic) for a few seconds. Acceptance: a bus parked across a lane stops the following AI vehicle instead of driving through it.
2. **Contact events and `coll_energy`.** `player_collisions` emits `CollisionEvent { other, point, normal, energy }` (energy = 0.5·m·v_n², v_n is the removed normal velocity component). The bridge writes `coll_energy` and raises the script trigger `collision` (the scripts of the 21C already define `{trigger:collision}`). Damage reads the same event: a per-vehicle `Damage` component (sum of energy over a threshold) for the later `[malfunction]`/dent features; no mesh deformation.
3. **Sound.** The events feed the audio layer's one-shot channel (`ev_*` events from the script, plus a generic impact sample chosen by energy class when the vehicle has no collision sound). Missing sample: log ERROR and stay silent (no placeholder).
4. **Trailers pushed.** `player_collisions` currently pushes the head only. Distribute the push over the chain: a contact on part *k* moves part *k* by the push and re-solves the chain constraint (the coupling distance to its neighbours, as `articulation` does for the pose) so that the head follows; contacts of the trailer run through the same loop with the part's own `Obb`. Prerequisite: the articulated pose solver gets a "pushed pose" input instead of only the simulated pose.
5. **Protected collision meshes.** `[collision_mesh]` of scrambled `.sco` cannot be decoded; use the object's own `[boundingbox]` if it has one, else the bounding box of its visual mesh bounds with the object's scale (marked `approx` in the index and logged once per object, not silently).
6. **Trees.** Trees have no collision mesh in OMSI either; give trunks a small round box from the tree definition (`.sco` of the tree: trunk radius from the mesh bounds at 1 m height) only if the map file marks them solid; otherwise leave them ghosts, as in OMSI.
7. **Per-wheel kerb climbing and terrain roll/pitch.** Sample the ground under each wheel (`GroundIndex::height_at`), derive pitch/roll from the wheel heights, let the body ride on the springs (a second-order lag) and block only when a single wheel meets a step above the climb limit (0.3 m). This also fixes the sinking into embankments next to platforms (the 9.6 m apron falling to 1.4 m terrain).

## Not yet

Everything above; pedestrians; `[crashmode_pole]`.

## Under a bridge (2026-10-22, Ahlheim Hauptbahnhof towards Frankenstraße)

A bus stopped under the railway bridge at the station. Three causes, found with `OPTS_DBG_COLLISION=1` (logs what pushes the player) and a minimap-following test drive:

* **The ground step saw the deck.** The step check compared the ground just outside the footprint with the ground inside it; where the road layer has no sample (the apron of the bus station) the only layer was the railway deck 6 m up, a "step" of 6 m that blocked the bus. A ground higher than the vehicle's own height is a deck it passes under, no step (`ground_step`, `clearance`).
* **The ground height climbed the deck.** `height_at` knew the terrain only where no road layer existed, so the same apron read the deck's 6 m and the bus rose onto it wheel by wheel. The terrain is now a layer too: it wins when it is clearly nearer the vehicle's height than the nearest road layer (0.3 m margin), so a vehicle on the deck stays on it and one on the ground stays on the ground.
* **One box per collision mesh.** A bridge's mesh reaches from the abutments to the deck, its box filled the passage. A mesh wider than 8 m is now cut into 2 m plan columns that keep only the height intervals their triangles occupy, equal neighbours merged (`collision/mesh_boxes.rs`): the deck is a slab at 5 m, an abutment face a full-height wall, the road between them free. (Not the cause at this bridge, but it would have blocked the next one.)

