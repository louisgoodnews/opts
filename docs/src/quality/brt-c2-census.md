# BRT C2 Part Census (M-0138)

*Facts about `Vehicles/BRT_C2-Solo/C2_2T.bus` read by `crates/opts-game/examples/vehicle_census.rs` (no GPU; run on the installation of 2026-10-20) and by the scene trace of the running game. No OMSI asset is part of this repository; the table is the tool's output, summarised.*

## How to reproduce

```
cargo run --release -p opts-game --example vehicle_census -- "<OMSI root>" "<OMSI root>/Vehicles/BRT_C2-Solo/C2_2T.bus" > census.tsv
```

One row per material slot of every `[mesh]` of the model config: lod, mesh file, status (`ok`, `FILE_MISSING`, `PARSE_ERROR`), layout (`plain`, `exact`, `UNREADABLE`), vertices, triangles, viewpoint flags, shadow flag, number of `[visible]` rules and animations, slot, texture name, the file the engine's own search finds (`NOT_FOUND` otherwise), the `[matl_alpha]` mode of the config, the `.o3d` diffuse alpha, the emissive colour, the `[alphascale]` variable. The summary goes to stderr. The scene trace comparison is `trace.find` with `file:"*BRT_C2-Solo*" full:true` in a game started with `--trace-scene=full` (the vehicle record lists every built mesh with its material notes).

## Result for the solo C2 (`modeldata_C2_2T.cfg`)

| Fact | Value |
| :--- | ---: |
| `[mesh]` entries (the config has no `[LOD]` groups: all are one level) | 545 |
| files not found / not parseable | 0 / 0 |
| material slots | 653 |
| layout | 653 plain (none keyed, none scrambled) |
| named textures the engine's search does not find | 0 (the two lamp textures of MI-0038 are not used by this variant) |
| meshes the scene trace shows built | 544 of 545 |
| the one not built | `Karosserien/2T/C2_2T_Schatten.o3d`, the shadow blob (`isshadow`, texture `Shadow.tga`): OPTS casts real shadows |
| built but hidden at rest | `Boden_dirt.o3d` (an alpha-scaled dirt overlay, `Dirt_interior`) |
| viewpoint flags | 380 meshes `3` (outside and inside), 165 `0` (always) |
| `[matl_alpha]` of the slots | 432 none, 217 mode 2, 4 mode 1 |
| slots whose `.o3d` diffuse alpha is 0 | 238 (6 more are 0.60: the partition glass) |

## The diffuse alpha 0 finding (MI-0064)

Of the 238 zero-alpha slots, 16 have an `[alphascale]` variable and 222 do not. The 222 are the interesting ones:

| Group | Slots | `[matl_alpha]` | Texture alpha |
| :--- | ---: | :--- | :--- |
| the driver terminal's buttons and screens (`Buttons.tga`, about 150 meshes: `Select_*`, `Ticket_*`, `Einstellung_*` …) | ~150 | 2 (clickboxes 1) | a mask, mean 0.62 |
| exterior and interior glass (`Fenster.tga`, `Fenster_int.tga`, `Innenraum_Fahrerplatz_Glas_hell.tga`) | ~35 | 2 | constant 0.61 / 0.30 / 0.25 |
| decals: brake light, vehicle number, wheelchair pictogram, floor ambient occlusion, bubbles | 6 | 2 | masks |
| door leaves, opening windows, ticket validator, timetable, list (`C2_2door_01*`, `Schedule.bmp` …) | ~25 | none | colour data |
| rain layers (`regen.tga`) | ~12 | none | hidden by design |

The authors set the key `[matl_alpha] 2` on all the visible ones and leave the exporter's opacity field at 0; the textures hold the transparency they meant (a 4×4 glass texture of RGB (0, 5, 8) with alpha 155 is a dark tinted pane). With the diffuse alpha multiplied in, all of it drew invisible. The same pattern is in 5 other add-on families of a 90-vehicle sample (`MAN_NewLionsCity` 382 slots, `C2-Fam_Vol-1` 150, `Citybus_S31x` 147, `GPM_C2` 77, `GPM_MAN_LionsCity_M` 51, `IvecoBus_citybus_family` 34) and nowhere in the Kajosoft Citybus 530 (1,919 slots, none). Evidence that OMSI behaves the same: add-ons of this popularity with windows that drew clear would not exist.

**Rule now (`material::textured_base_alpha`):** a textured material with `[matl_alpha]` 1 or higher takes its transparency from the texture alone; a material without the key keeps the diffuse alpha (below 1 it blends, 0 stays invisible: the mutation test T03 observed that for materials without the key, and the 12 sampled add-on slots without the key and with alpha 0 are rain layers and similar hidden overlays).

**Effect checked on the running C2** (Ahlheim 5, eight views and a top view before and after): the side, door and windscreen glass is a dark translucent pane with its frames, the wheelchair and pram pictograms show on the middle door, the interior can be seen through tinted glass instead of an open side. The touchscreen meshes and the decals draw (their `[visible]` rules decide when).

## Not yet done (M-0138 continues)

- the viewpoint flags (`OUTSIDE`/`INSIDE`/`AI_BUS`) are read, but their use per camera was not verified part by part;
- the cockpit parts flagged `UV_CHAOS` (MI-0038) one by one;
- the door, wiper, indicator and instrument animation list;
- the display textures (waiting for M-0100);
- the project lead's look.
