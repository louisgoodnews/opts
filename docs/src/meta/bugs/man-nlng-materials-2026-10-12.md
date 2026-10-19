# MAN NL/NG material artifacts (P-1012)

Status: Phase A (diagnostic) complete, root cause candidate identified, waiting for approval.
Vehicles: `MAN_NL_NG` (EN92 solo, GN92 main + trail; the player vehicle "MAN NG272 - GN92").
Tools (this branch): `o3d_materials`, `vehicle_material_audit`, `scrambled_uv_report` (`crates/opts-omsi/examples/`).

## Preconditions
1. **Reproduced** with GN92 main + trail on Aachen Linie 33 (cockpit, passenger and external cameras, release build of
   `fix/vehicle-animations` 29b6690). The three screenshots of the task arrived after the first draft of this report
   and are covered in A1 (they match the views reproduced here).
2. Reproducible: the same places on every start (the decode is deterministic and cached).
3. **Comparison vehicles:** MAN_SD200 SD77 (5 of 165 meshes keyed), MAN_NewLionsCity 21C (0 of 797 keyed). Neither shows
   the artifacts.
4. The `--debug-wheel-vars` overlay still works. A material debug flag is not needed so far (`--log-level debug` logs
   `model part` per slot with its texture).
5. Branch `fix/man-nlng-materials` from `bevy-0.20` (worktree `../opts-p1012`); P-1011 stays on `fix/vehicle-animations`
   (tests of that branch pass; the full suite on the new branch has not been run in this phase).

## A1 - artifacts
| View | Artifact | Where | Class |
|---|---|---|---|
| Cockpit | pink/purple glyph patches | steering wheel rim | wrong UV region (the rim samples labels of the `GN92_Panel.bmp` atlas) |
| Cockpit | garbled dashboard faces, odd colours | panel meshes (`GN92_Panel*`) | wrong UV region |
| Passenger | red/yellow patches on grab poles | `GN92_main_innenraum` slot 2 (`GN92_1.tga`, 16 faces) | wrong UV region (samples the exterior livery atlas where the poles' colour is) |
| Passenger | ceiling, side panels | `Decke.bmp` slot | render correctly in this run |
| Cockpit | missing switch/lever parts | 33 skipped meshes (`GN92_Panel_Hub_Up`, `Gangwahl_*`, ...) | not decoded (logged `scrambled; skipped`), not a material problem |

### The three task screenshots
| Screenshot | Artifact | What the pixels are |
|---|---|---|
| Cockpit | pink/purple glyphs on the wheel rim | patches of the `GN92_Panel.bmp` atlas (labels, pink LEDs) |
| Cockpit | the driver's right console (IBIS keypad, ticket printer) covered in blue/yellow/black shards | parts of the panel atlas stretched over the wrong faces |
| Interior (looking up in the driver area) | large panels showing yellow/black stripes, wood grain, a vent grille and the red emergency button | **not the exterior livery**: these are regions of the *interior* atlas `GN92_2.bmp` (yellow stripe along its top edge, wood panels, vent, red button), shown at the wrong place and scale; `GN92_1.tga` (the livery base) is almost white |
| External | none | renders correctly |

So the interior "livery" report is the same class as the cockpit: the **right texture** with the **wrong UV region**.

No artifact is a missing-texture fallback, a wrong slot index or a geometry overlap that I could show.

## A2 - material table (cfg)
The `[matl]`, `[matl_change]` (`texture, n, variable`: *n-th material using that texture*, implemented in
`find_override`) and `[matl_nightmap]` entries of the artifact meshes agree with the `.o3d` slots: the interior mesh has
7 slots (`GN92_2.bmp`, `Decke.bmp`, `GN92_1.tga`, 4 x `N92_3.tga`) and its four `[matl_change] N92_3.tga 0..3` entries
address the four `N92_3.tga` slots; `GN92_Lenkrad.o3d` has **one** slot (`GN92_Panel.bmp`) and the cfg needs no `[matl]`.

## A3 - O3D table vs cfg
`vehicle_material_audit model_GN92_main.cfg`: 217 meshes, **2 slot-count mismatches** (`GN92_main_wagenkasten_dreck`,
`GN92_articul_bellows_dirt`: two cfg entries for one slot, dirt overlays), no per-face index beyond the table. The count
mismatch is not the cause of the artifacts.

## A4 - texture resolution
All textures of the artifact meshes exist (any extension): `GN92_Panel.bmp`, `GN92_1.tga`, `GN92_2.bmp`, `Decke.bmp`,
`N92_3.tga`, `N92_N.bmp`, `envmap_*.bmp`. Missing names are (a) text-texture / registration textures filled by scripts
(`D_kennzeichen.tga`, `D_wagennummer.tga`), (b) **separator lines taken as texture names**: 22 meshes log
`texture not found: =====...`, `-----...`, `.....`, `Rauchsystem`, `\\S:1`: a keyword with an *empty* parameter line
(for example `[matl_transmap]` followed by blank lines and `#####`) makes the parser take the next non-blank line as the
value. This is a real but separate parse bug (no visual effect today, the maps are empty); it is listed for a
follow-up.

## A5 - comparison
| Model | meshes | keyed (scrambled) |
|---|---|---|
| MAN_NewLionsCity 21C | 797 | 0 |
| MAN_SD200 SD77 | 165 | 5 |
| MAN_NL_NG EN92 | 208 | **208** |
| MAN_NL_NG GN92 main | 217 | **217** |
| MAN_NL_NG GN92 trail | 40 | **40** |

The MAN NL/NG family is shipped entirely with the exporter's vertex scrambling, so every mesh depends on the decoder
(`docs/src/formats/binary/o3d.md`, Addendum 5): positions are recovered by optimisation (184 of 217 GN92 meshes
accepted, the other 33 skipped) and texture coordinates by `uvfix`, which restores only about 77 % of the UVs exactly
on the twin data (23 % keep the scrambler's jitter of up to ~0.9 in u and v).

Evidence that the artifacts are residual UV jitter (`scrambled_uv_report`):
- GN92 steering wheel after decoding: median UV (0.859, -0.841); 213 of 2,263 vertices (9.4 %) are more than 0.2 away from
  it and the range is u 0.47..1.82, v -1.71..0.20, i.e. outside the atlas region the rim belongs to; the pink patches
  are those vertices.
- GN92 tachometer needle (9 vertices): UV range u 0.324..1.227, v -0.914..-0.104. The plain SD77 needle of the same
  design maps all vertices to one texel (u 0.323..0.324, v -0.957..-0.956).
- Plain SD77 steering wheel: every vertex has the identical UV (0.971, -0.511).
- The decoder reports 0 % unresolved vertices for these meshes, so its own report cannot flag them.

## A6 - answer
- **Classes:** wrong UV region (steering wheel rim, dashboard faces, grab poles). No missing-texture fallback, no wrong
  material index, no geometry overlap found.
- **Material count mismatch:** only 2 dirt-overlay meshes; unrelated.
- **Texture resolution:** fine; one separate parse bug (blank keyword parameter takes the next line).
- **Difference from the working references:** they are not keyed (NLC) or almost not keyed (SD77); GN92 is 100 % keyed.

**Root cause candidate for Phase B:** the artifacts are not a material-binding fault. They are undecoded texture jitter
on scrambled meshes (Candidate 5/UV, not 1-4). Whether the interior "exterior livery" is the same effect (poles sampling
the livery atlas) is answered by the screenshots: the interior panels show `GN92_2.bmp`, the interior atlas, so the cause is the same as in the cockpit.
