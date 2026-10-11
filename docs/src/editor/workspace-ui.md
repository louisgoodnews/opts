# Workspace and User Interface

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Toolkit** | `opts-gui` (Material Design 3 tokens and widgets, [GUI design system](../architecture/gui-design-system.md)) |

## 1. Layout

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│ File  Edit  View  Tools  Validate  Play  Window  Help      project ▾ │ undo redo │ save   │
├────────┬─────────────────────────────────────────────────────────────┬───────────────────┤
│ Tool   │                                                             │ Inspector         │
│ palette│                       3D viewport                           │  (selection or    │
│        │          (game renderer, editor overlays, gizmos)           │   file schema)    │
│ select │                                                             ├───────────────────┤
│ move   │                                                             │ Outliner          │
│ rotate │                                                             │  tiles ▸ objects, │
│ spline │                                                             │  splines, rules…  │
│ paint… │                                                             │                   │
├────────┴───────────┬─────────────────────────────────────────────────┴───────────────────┤
│ Asset browser      │ Validation · Console · Source view · Timetable · Profile (tabs)      │
└────────────────────┴─────────────────────────────────────────────────────────────────────┘
```

* Panels dock, float, hide and persist per *workspace preset* (**Map**, **Vehicle**, **Transport**, **Assets**, **Source**); presets are stored in the user settings database like other preferences.
* The window uses the M3 theme (light/dark, accent) of the game; the editor adds a *dense* spacing scale (`SpacingScale::Dense`) because it shows many fields.
* All text is localised through the same string tables as the game (German and English first).

## 2. Panels

| Panel | Content and behaviour |
| :--- | :--- |
| **Viewport** | Game renderer with editor overlays. Modes: *Shaded*, *Wireframe*, *Paths* (lanes, pedestrian paths, rail), *Rules* (traffic rules, speed limits), *Terrain grid*, *IDs*, *Validation* (elements with problems tinted). Free camera, orbit, top view (orthographic), *Walk* (eye height 1.7 m) and *Vehicle cab* camera. |
| **Outliner** | Tree of the open project: documents → element groups → elements. Search, filters by type, visibility/lock toggles per group, drag to re-parent attachments. Selecting syncs with the viewport. |
| **Inspector** | Properties of the selection, generated from the [schema](architecture.md#5-schema-registry): labels with units, typed fields, ranges as sliders, file pickers that show the resolved path, colour pickers, per-field *raw value* toggle, links to the specification page, per-field reset to the original. Multi-selection edits common fields (mixed values show `—`). |
| **Asset browser** | The installed catalogue (SQLite, [catalogue schema](../architecture/catalogue-schema.md)) plus the project overlay: scenery objects, spline profiles, vehicles, characters, textures, scripts with thumbnails (rendered on demand and cached), search by name/group/author/path, *used by* and *uses* lists, drag into the viewport to place. Relocated files ([`o3d.md`](../formats/binary/o3d.md), Addendum 9) are marked. |
| **Validation** | The [rule results](validation.md) grouped by severity and rule, each with *Go to*, *Explain* (specification excerpt) and *Fix* where an auto-fix exists. Live while editing. |
| **Console** | Log of commands, saves, loader messages (`ERROR`/`WARN` from the loaders), and a command line for control-protocol commands (`editor.select id=…`). |
| **Source view** | The selected file as text: syntax highlighting per format, keyword completion and parameter hints from the schema, gutter markers from validation, side-by-side diff against the original or the last save, edits are parsed on the fly and applied as commands when valid. |
| **Timetable** | Grid and diagram editors for trips and lines ([transport content](content-transport.md#3-timetables)). |
| **Profile** | Cross-section canvas for spline profiles ([assets](content-assets.md#3-spline-profiles-sli)). |
| **Variables** | Live table of vehicle/scenery script variables with sliders and "freeze" toggles, to pose animations and test visibility rules without running the script ([vehicles](content-vehicles.md#4-animation-and-variable-preview)). |

## 3. Tools

| Tool | Key | Behaviour |
| :--- | :---: | :--- |
| Select | `Q` | Click, box (`Shift` adds, `Ctrl` toggles), select-similar, select-by-rule; hover shows the element, id and file. |
| Move / Rotate / Scale | `W` / `E` / `R` | Gizmo with axis/plane handles; numeric entry; *surface snap* puts objects on terrain or spline surface; heading snaps to 15°; heights relative to terrain or absolute (`[absheight]`) — the inspector says which. |
| Place object | `P` | Pick an object in the browser, click to place, `Shift` for repeated placement, `Tab` cycles variants; preview ghost with validation tint. |
| Spline | `S` | Click control points; the tool chooses straight or arc segments (`radius`, `length`, gradient per segment) and shows the ribbon live; snaps to existing spline ends (connection `conn_id` set automatically) and junction arms; right-click ends. |
| Terrain | `T` | Raise, lower, smooth, flatten to level/along spline, noise, import heightmap; brush radius/strength; the 61 × 61 grid per tile is drawn; edits across tile borders stay consistent (shared edge heights). |
| Paint ground | `G` | Per-tile layer masks (`[groundtex]` layers) with soft brush; layer list shows textures and repeats. |
| Path / lane | `L` | Draw `[path]` lanes of a junction object (type, width, direction), connect ends (geometric snapping), assign traffic lights. |
| Rule | `U` | Select a lane/spline, attach `speedlimit`, `priority`, `trafficdensity`, vehicle-type restrictions; arrows show the effect. |
| Measure | `M` | Distance, slope, clearance, bounding box. |
| Stop / station | `B` | Place a bus-stop object, link it to a `Busstops.cfg` entry and a station. |

Commands: `Ctrl+Z`/`Ctrl+Shift+Z` undo/redo, `Ctrl+S` save, `Ctrl+Shift+S` save all, `Ctrl+D` duplicate, `Del` delete, `F` frame selection, `Ctrl+F` search, `F5` Play from here, `F10` validation, `Alt+Z` cycle shading mode. Every tool and key is listed in *Help → Shortcuts* and can be rebound.

## 4. Navigation and snapping

* Free camera uses the game's controls (`WASD`, mouse look, wheel speed); orbit around the selection with the middle mouse button; the tile coordinate and OMSI world coordinates (x east, y north, z up) are shown in the status bar ([coordinates](../architecture/coordinates.md)).
* Snaps (toggle each, `Ctrl` inverts while dragging): grid (0.25, 0.5, 1, 5 m), terrain grid vertices, object origins, spline ends and control points, junction arm ends, lane ends, bounding-box faces, 15° angles.
* The map can be streamed far beyond the editable area: *editable* tiles are those loaded in the project; neighbouring tiles of the installation are shown read-only and greyed unless *Add tile to project* is chosen (copies the file into the overlay).

## 5. Multi-file projects

A project usually spans many files (a map: global.cfg + hundreds of tiles + lists + timetables). The outliner lists documents with *modified*, *error* and *read-only* badges; *Save all* writes the modified ones. Opening an element of an unloaded tile loads that tile in the background. Closing a document discards its view state, not its edits (they stay in the project until saved or reverted).

## 6. Accessibility and ergonomics

* Full keyboard operation of panels (tab order from `Focusable` in `opts-gui`), visible focus ring, no information by colour alone (validation uses icons and text), WCAG AA contrast from the M3 schemes, scalable UI (80–200 %).
* Long operations (open map, validate project, bake light map) show progress and are cancellable; the editor never blocks the viewport for more than one frame budget ([architecture §7](architecture.md#7-threading-and-performance-budget)).
* Destructive actions (delete tile from project, install into OMSI) ask for confirmation with a precise list; everything else is undoable instead of confirmed.
