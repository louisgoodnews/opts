# Editing and Creating Vehicles and Characters

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Formats** | [`.bus`/`.ovh`](../formats/text/vehicles/bus.md), [`model.cfg`](../formats/text/vehicles/model_cfg.md), [`paths.cfg`](../formats/text/vehicles/paths_cfg.md), [`passengercabin.cfg`](../formats/text/vehicles/passengercabin_cfg.md), [`sound.cfg`](../formats/text/vehicles/sound_cfg.md), [scripts](../formats/text/vehicles/osc.md), [`.cti`](../formats/text/repaints.md), [`.hum`](../formats/text/humans.md) |
| **Phase** | E4 |

## 1. Vehicle project layout

A vehicle is a folder `Vehicles/<Name>/` that the editor keeps in the OMSI layout:

```
<Name>.bus  (+ .ovh for AI variants, *_DEU.dsc … descriptions)
Model/<model>.cfg  Model/<mesh folder>/*.o3d
Texture/…   Script/…   Sound/…   Model/paths_*.cfg   Model/passengercabin_*.cfg
<Depot>.hof ... (only when the vehicle ships its own depot files)
```

*New vehicle* offers **Bus**, **Articulated bus** (main + trailer `.bus` linked by `[couple_back]`/coupling points), **Rigid truck**, **Tram/rail vehicle**, **Car** (`.ovh` for AI plus a parked `.sco`, §6) and **Trailer**; each template starts from the minimal files that load and are valid, not from copied third-party content.

## 2. The vehicle manifest (`.bus`)

The *Vehicle* inspector groups the documented keywords:

| Group | Keywords | Editor support |
| :--- | :--- | :--- |
| Identity | `[friendlyname]` (manufacturer, model, **default repaint**), `[description]` … `[end]`, localised `.dsc` | text fields; the third `[friendlyname]` line is the default livery shown in the vehicle selection ([repaints](#5-repaints)) |
| Fleet numbers | `[number]` `.org`, `[registration]` modes | picker for `.org` files; preview of a drawn number |
| State and type | `[type]`, `[ai_deltaheight]`, `[script_share]` | enumerations with explanations |
| Model, sound, scripts | `[model]`, `[sound]`, `[script]…`, `[varnamelist]`, `[constfile]` | path pickers that resolve (relocation hints), "open" buttons |
| Cameras | `[add_camera_driver]`, `[add_camera_pax]`, reflection cameras | place a camera in the viewport (gizmo), live preview of the view, `std_camera` choice |
| Physical properties | `[mass]` (4 lines: tonnes + legacy inertia), `[momentofintertia]`, `[schwerpunkt]`, `[boundingbox]` (**full** extents + centre, [Addendum 9](../formats/binary/o3d.md)), `[rollwiderstand]`, `[rot_pnt_long]`, `[inv_min_turnradius]` | *Fit bounding box to model*, centre-of-mass handle, turning-circle preview, mass/inertia estimate from the model volume (suggestion only) |
| Axles | `[newachse]` with wheels, suspension, drive/steering/brake groups | axle gizmo on the chassis, wheel radius from the wheel mesh, symmetric pairs, **test rig**: drop the vehicle on a plane and watch the suspension (uses `opts-sim`) |
| Attachments | `[new_attachment]` | tow hitch, mirror arms etc. |
| Coupling | `[coupling_front]`, `[coupling_back]`, `[couple_back]` chain, character couplings | coupling points as gizmos, chain assembly preview with the trailer |

Fields the specifications flag as open (the secondary bounding box, reflection camera parameters) are shown as raw values and kept.

## 3. Model (`model.cfg`) and meshes

* **Mesh list.** The outliner shows the LOD tree (`[LOD]` blocks: the first block of a file is not always the close-up level — parked-vehicle models list a decal-heavy level first), each `[mesh]` with its `[matl*]` modifiers, `[viewpoint]` (outside/inside), `[shadow]`, `[visible]` (variable-driven visibility), `[illumination_interior]`. Drag meshes between LODs; the *LOD distance* handle uses the camera distance metric.
* **Animations (`[newanim]`).** Each animation block (origin translation/rotation, then `anim_rot`/`anim_trans` by a script variable with a factor) is edited with a pivot gizmo in the viewport and a variable slider ([§4](#4-animation-and-variable-preview)); order matters (first step innermost) and is shown.
* **Lights.** `[light_enh*]`/`[light_enh_2]`, `[interiorlight]`, headlight `Spot_Select` entries with cone gizmos; night preview.
* **Effects.** `[smoke]` emitters (exhaust, rain spray at wheels), `[particle_emitter]`.
* **Text and screens.** `[texttexture]`/`[texttexture_enh]` (destination boards, clocks, number plates): font, size, variable; preview with the same renderer as the game ([text textures](../architecture/text-textures.md)).
* **Repaint hooks.** `[CTC]`/`[CTCTexture]` blocks list the colour-scheme slots; see §5.
* **Mesh validation** specific to vehicles: every mesh referenced exists and is plain (protected meshes of other authors are flagged), wheel meshes centred on their `[newanim]` pivots (the corpus shows 8,007 of 8,012 wheel meshes within 0.35 m), body mesh inside the `[boundingbox]` (the box covers the body without roof equipment and undercarriage, so a margin is allowed), texture sizes and formats.

## 4. Animation and variable preview

The **Variables** panel lists every script variable the model, the `[newanim]` blocks, the visibility rules and the sound/light definitions use (from `varnamelist` and the model) with type, default and *used by* links. Moving a slider drives the variable directly (doors 0→1, steering −1→1, wheel rotation, indicator light) so the author sees animation and visibility without the script; *Run script* instead executes the real script set in `opts-sim` against the stub vehicle, with the same diagnostics the game logs. Both modes can be combined: freeze some variables, let the scripts compute the rest.

## 5. Repaints

* **Colour schemes.** The `[CTC]` blocks (name, texture folder, default index) and `[CTCTexture]` slots of the model are listed; the folder's `.cti` files define the repaints (`[item]` per slot, `[setvar]` equipment options).
* **Livery editor.** Choose a slot, a texture, a name; items with the same name form one repaint. The panel previews the vehicle with each repaint and shows the `[setvar]` effects (e.g. `vis_wimpel_invisible`). The default repaint in `[friendlyname]` must exist or the validator warns.
* **Repaint kit.** From the template set (`.rpc`: Basic, Alpha, Mask, Add, Multiply bitmaps) the editor composes the final texture from user layers and writes `.dds` and the `.cti` entry in one step ([assets](content-assets.md#2-textures-and-materials)).
* **Ads and destination boards** use further `[CTC]` categories (not named `Colorscheme`); they are edited the same way and listed separately.

## 6. Cars and AI vehicles

* **`.ovh`** files share the `.bus` grammar ([`misc-types`](../formats/text/misc-types.md)); the editor creates the AI variant from the player vehicle (drops the cockpit, keeps physical properties, LOD-reduced meshes `_AI.cfg`).
* **Parked cars** are a `.sco` with `[model]` pointing to a vehicle-style `model_parked.cfg` (plain body at `[LOD] 0`), listed in the map's `parklist_p.txt` ([lists](../formats/map/lists.md)); the *Car* template writes all three.
* **Registration plates** and colours: `.org`/`[registration]`, `[CTC]` colour schemes.

## 7. Characters (`.hum`)

The **Character** editor edits `[humangeom]` (stance, height), the 22 `[links]` joint pivots (dragged on the rig in the viewport, mirrored left–right), `[walk_param]` (stride, arm flare, arm swing, hip and waist sway; the pedestrian gait previews immediately, [pedestrians](../simulation/pedestrians.md)), `[seatheight]`, `[mass]`, `[voice]`, `[age]` and the model reference. It validates that the mesh has the 13-bone weight chunk and that bone pivots match `[links]`. A pose preview shows *standing*, *seated* and *walking*.

## 8. Vehicle test loop

*Test drive* starts the game session with the project's vehicle on a chosen map at the camera position (`F5`); the editor can also drop the vehicle onto a flat test plane with a range of obstacles (kerb, slope, speed bump) to inspect suspension and steering without loading a map. Saves are hot-reloaded into the running vehicle (the existing vehicle reload path, [hot reload](../architecture/hot-reload.md)).
