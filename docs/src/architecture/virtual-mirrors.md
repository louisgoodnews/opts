# Virtual Mirrors

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-22) |
| **Crates** | `opts-bevy` (`mirrors/virtual_view.rs`, `mirrors/update.rs`), `opts-game` (`hud/virtual_mirrors.rs`) |
| **Related** | [camera system](camera-system.md), [reflections research](reflections-research.md), [`.bus`](../formats/text/vehicles/bus.md) (`[add_camera_reflexion_N]`) |

The real mirror panes of a vehicle are small and often out of the driver's view. The virtual mirrors show the picture of a mirror camera as a panel on the screen, at the edge where the mirror sits.

## Use

In the driver's view (key 2):

1. **Tab** opens the mode. The areas of the **left mirror** and the **two right mirrors** (main and wide-angle) appear as dark boxes with their names. A vehicle that lacks a mirror shows no box for it.
2. **Click an area.** The mirror's camera streams into a panel there (a picture twice the size of the pane picture, rendered every frame). Click it again to switch it off.
3. **Tab** again hides the areas of the mirrors that are off. A panel that is on stays while the vehicle is driven.

Leaving the driver's view hides the panels (they return when it is entered again); the viewpoint editor hides them while it is open. A mouse that steers (`MouseDriving`) is let go while the mode is open and given back when it closes.

The picture is mirrored left to right, so that something that comes up on the left of the vehicle appears on the left of the panel, as in a mirror.

**The viewpoint editor moved to Shift + Tab** (inside the editor Tab still saves and closes it).

## How a mirror is chosen

The cameras are the mirror cameras of `opts_bevy::mirrors` (one per `reflexion<N>.bmp` material; the camera of a `.bus` `[add_camera_reflexion_N]` tag sits where the mirror is and looks where the tag says). A mirror gets a slot by the position of its tag in the vehicle's frame (the pane's centre when the vehicle has no tag), so a mirror works whether or not its pane is visible in the current variant:

* only **side mirrors** count: at least 0.8 m from the centre line (the interior mirror does not) and within 1.5 m of the foremost one on their side (cameras along the body do not);
* **Left** is the highest on the left, **Right upper** and **Right lower** the two highest on the right.

On the MAN Lion's City these are the tags 0 (left), 2 (right wide-angle, upper) and 1 (right main, lower); the interior mirror (5), the centre camera (3) and the side camera (4) get no slot.

## What it costs

Nothing while no mirror is on. A mirror that is on renders every frame at twice the picture size (at most 1024 a side), whatever the light quality's `mirror_count`; with `mirror_count` 0 the cameras exist only for the mirrors that are on.

## Tests

`opts-bevy` `mirrors/tests/virtual_view.rs` (slot choice with the Lion's City data, a vehicle with one right mirror, stream size, toggling); in the running game `ui.click` on `hud.virtual_mirror.<0..2>` after `input.key Tab` in view 2.

## Live mirrors of other vehicles

The player's mirrors have a camera each. The mirrors of other vehicles (AI traffic, parked vehicles) would keep the static `reflexion<N>.bmp`, a baked road scene. `opts_bevy::mirrors::ambient` gives the **nearest** of them a live picture:

* every mesh with a mirror material is marked once when it appears (`AmbientPane`: geometry, UV rectangle, own static material); the player's own panes are skipped;
* `LightQuality::mirror_count` cameras (2 up to light level 3, none above) serve the panes that are visible, within 30 m, in front of the main camera and facing it, nearest first; a pane keeps its camera while it stays among the nearest;
* the pose is the geometric reflection of the viewer's ray (no `.bus` tag for these), the picture is 192 pixels square and renders every second `mirror_every`-th frame, staggered by rank; a pane that loses its camera gets its static material back;
* `sim.get` reports `reflections.mirrors.ambient`, the number of served panes.

Only models whose mirror is a separate pane with a `reflexion<N>` texture are covered: the AI variants of the buses (for example the Lion's City `KI_*` models). Passenger cars bake the mirror into the car texture and keep it.

Tests: `opts-bevy` `mirrors/tests/ambient.rs` (nearest two, out of range or behind the camera, budget 0, a camera keeps its pane) and, with an install, `tests/mirror_content.rs` (real Lion's City meshes of another vehicle get two cameras).
