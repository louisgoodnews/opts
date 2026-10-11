# Editing and Creating Assets

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Formats** | [`.o3d`](../formats/binary/o3d.md) / [`.x`](../formats/binary/x.md), [textures](../formats/textures.md), [`.sli`](../formats/text/sli.md), [`.sco`](../formats/text/sco.md), [scripts](../architecture/script-vm.md), [fonts](../formats/text/fonts.md) |
| **Phase** | E3 |

## 1. Meshes

**Reading.** Plain `.o3d` (and `.x`) meshes open with the game's loader. Scrambled meshes open *read-only* ([README §6](README.md#6-protected-content-and-clean-room-rules)): the viewport shows OPTS's reconstruction, the inspector marks it *protected*, every edit tool refuses it with the reason.

**Writing.** The mesh writer produces **plain** `.o3d` (header key `0xFFFF_FFFF`), version 7 by default (the version most vehicle meshes use; v1/v4/v5 only when the user targets an older tool). Chunks: `0x17` vertices (position, normal, UV), `0x49` faces (16-bit or 32-bit indices by vertex count, flag bit 0), `0x26` materials (diffuse RGBA, specular, emissive, power, texture names), optional `0x79` matrix (identity — OPTS does not apply a matrix, [Addendum 4](../formats/binary/o3d.md)) and `0x54` bone weights for characters. The layout is the documented one; the writer is validated by (a) re-reading with the game's parser, (b) comparing the parse of re-written *plain* corpus meshes with the original, (c) loading in real OMSI 2.

**Import.** glTF 2.0 and OBJ (+MTL) become `.o3d`: Y-up right-handed → OMSI's frame (x lateral, y up, z forward, left-handed: the loaders' conversion inverted, with winding reversed), units metres, materials → `[matl]` entries (base colour texture → texture name; alpha mode → `[matl_alpha]`), multiple meshes → material slots (OMSI draws one mesh per material slot), smoothing normals optional (OMSI meshes store vertex normals; hard edges need duplicated vertices — the importer splits by angle). Skinned glTF is mapped onto the 13-bone character rig (bone-name table, weights normalised to four influences).

**Checks while importing:** triangle count and vertex budget (per object class), degenerate and flipped triangles, non-manifold edges for collision meshes, texture coordinates outside `0…1` flagged as tiling, units (a 2 m door that measures 2,000 is flagged), pivot at the object's ground contact.

**Mesh tools** (kept small on purpose): move/rotate/scale the whole mesh, recentre pivot, merge/split by material, flip normals/winding, recompute normals, LOD generation by edge collapse (automatic, user reviews), collision mesh from convex decomposition, bounding box fit (writes the `[boundingbox]` of the `.sco`/`.bus` — full extents for vehicles, see [Addendum 9](../formats/binary/o3d.md)). Sculpting and UV unwrapping are external.

## 2. Textures and materials

* **Formats.** `.dds` (DXT1/3/5 and uncompressed), `.tga`, `.bmp`, `.png`, `.jpg` open; export to `.dds` with mip maps (BC1 without alpha, BC3 with alpha; BC7 not used because OMSI 2 predates it ❓ — a preference lists which formats the target OMSI accepts). The editor corrects known export quirks on write (24-bit TGA descriptor, power-of-two warning, sizes above 4096).
* **Alpha is not transparency by default.** OMSI uses a texture's alpha only where the material asks for it (`[matl_alpha]`, `[matl_transmap]`); paint textures carry other data there ([Addendum 9](../formats/binary/o3d.md)). The material inspector therefore shows alpha mode (*opaque / cutout / blend*) and whether the texture's alpha is used, with a preview of both.
* **Texture properties** `<texture>.<ext>.cfg` (surface type for sound/physics, [texture-cfg](../formats/text/texture-cfg.md)) are edited as a dropdown plus raw fields.
* **Repaint templates.** For vehicles, the editor ships a *Repaint* panel: the template bitmaps of a `.rpc` set (Basic, Alpha, Mask, Add, Multiply, [repaints](../formats/text/repaints.md)), a layered preview and an export of the final texture; registered liveries are written to a `.cti` ([vehicles](content-vehicles.md#5-repaints)).
* **Material slots** per mesh: texture, envmap (`[matl_envmap]` + strength), bump map (`[matl_bumpmap]` + strength), light map and night map (`[matl_lightmap]`, `[matl_nightmap]` with the switching variable), transmap, free textures, noZ flags, alpha scale variable. A preview ball shows day/night.

## 3. Spline profiles (`.sli`)

A **Profile** panel edits the cross-section ([`.sli`](../formats/text/sli.md)):

* **Graphical profiles** (`[profile]` + `[profilepnt]`): points (x lateral, z height, u texture coordinate, v gradient per metre) on a 2D canvas with snapping and mirror editing, texture index per profile, ribbon preview with tiling; winding direction (left to right) checked — a ribbon laid right to left is a bottom face ([winding rules](../meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md)).
* **Height profile** (`[heightprofile]`): strips that carve the terrain — shown as the terrain section under the profile; *terrain may not cover the road* is verified against the carve algorithm.
* **Paths** (`[path]`, `[path_2]`): lanes for road vehicles (0), pedestrians (1), rail (2), aircraft (3) with offset, height, width, direction; drawn on the canvas with arrows; the preview spawns a test vehicle/pedestrian on the lane.
* **Textures** list, `[onlyeditor]` helper profiles (invisible path-only splines), rail extras, terrain holes, `[patchwork_chain]`.
* **From scratch** the panel offers templates (two-lane road with sidewalks, tram track, footpath, fence, embankment) that are starting points, not stored defaults.

## 4. Scenery objects (`.sco`)

The **Object wizard** creates a `.sco` from a mesh and a purpose; every page maps to documented keyword groups ([`.sco` structure](../formats/text/sco.md)):

| Page | Keywords written |
| :--- | :--- |
| Identity | `[groups]` (editor tree), `[friendlyname]`, `[complexity]` |
| Behaviour | `[fixed]`, `[absheight]`, `[surface]`, `[nocollision]`, `[noshadow]`, `[NightMapMode]`, `[LightMapMapping]`, `[rendertype]`, `[onlyeditor]`, `[joinable]` |
| Geometry | `[mesh]` groups with their `[matl*]`, `[visible]`, `[shadow]`, LOD |
| Collision | `[boundingbox]` (with the unreliable-extents note: scenery boxes mix half and full extents between authors; the editor writes the convention the specification fixes and validates it against the mesh), `[collision_mesh]`, `[cog]` |
| Lights and effects | `[light_enh*]`, `[maplight]`, `[smoke]`, `[particle_emitter]` — previewed in the viewport |
| Animation and scripts | `[newanim]` (origin, rotation, translation by variable), `[script]`, `[varnamelist]`, `[constfile]`, `[sound]`, `[mouseevent]` |
| Traffic | `[traffic_lights_group]`, `[traffic_light]` + `[phase]`, `[use_traffic_light]`, `[path]` lanes, `[splinehelper]` arms |
| Strings | `[stringvarnamelist]`, text textures (`[useTextTexture]`, `[texttexture]`, fonts) |
| Trees and signs | `[tree]` billboards (textures, size ranges), sign templates with text textures |

Generators: **Junction** (arm angles/profiles → plate + paths + helpers), **Crossing**, **Bus stop** (shelter, pole, sign with string variables), **Parking row** (`car_park` markers), **Fence/wall** (spline profile + posts as attachments).

## 5. Scripts (`.osc`, variable lists, constants)

* The script editor is a text editor with the VM's **compiler diagnostics** inline (`opts_sim::script`: every one of the 6,062 corpus scripts compiles), symbol lookup (`{trigger:…}`, `{var}`, macros, `constfile` names), find-all-references, go to definition across the script set, and a *variable inspector* that runs the script on a stub vehicle/object ([Variables panel](workspace-ui.md#2-panels)).
* Variable lists (`varnamelist`) are edited as a table with usage counts (which scripts/meshes/animations read or write each name) so unused or undefined variables are visible.
* Stack-underflow-style problems that the VM reports at run time (`macro:*_frame`) are reproduced as diagnostics when the script is run in the preview; they point to the macro and operation.

## 6. Fonts, sounds, ticket packs

* **Bitmap fonts** (`.oft`: `[newfont]`, `[char]`): a generator rasterises a TrueType font to the two bitmaps (colour and alpha) and writes the character metrics; manual per-character adjustment; preview with the text-texture renderer used by the game.
* **Sounds** (`sound.cfg` of vehicles and objects, `.wav`): the cfg editor lists sound slots with their trigger variables and previews playback (once audio exists, [M-0027](../roadmap/phase-4/M-0027.md)); WAV import checks sample rate/channels.
* **Ticket packs** (`TicketPacks/<pack>/…`, [ticketpacks](../formats/text/ticketpacks.md)): ticket meshes, `.otp` tables (`[ticket_2]` price, ages, day tickets) and voice folders — table editors with validation of prices against the map's money system.
