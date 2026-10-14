# Text Textures (`[texttexture]`)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Scenery strings (M-0039); 🟡 vehicle text textures from script string variables (2026-10-12), their content waits for the host callbacks of [M-0093](../roadmap/phase-4/M-0093.md) |
| **Code** | `opts_core::formats::oft`, `opts_bevy::text_texture`, `opts-game` tile loader |
| **Last Updated** | 2026-10-10 |

## What OMSI does

A `.sco` declares text textures (`[texttexture]`, `[texttexture_enh]`), numbered by order of appearance. A mesh material marks one as its surface with `[useTextTexture] n` directly after the `[matl]` it replaces. The embedded `.sco` tutorial gives the fields: source string number (0 = first string), font name "as in the `.oft` file", width, height, mono (`0`) or original colours (`1`), R, G, B. `[texttexture_enh]` adds *orientation* (`0` centred, `1` left, `2` right; see [model.cfg](../formats/text/vehicles/model_cfg.md)) and a grid flag.

The string comes from the object **instance**: `MapObjectInstance::string_vars[n]` in the tile (a stop's name, a street name). The font is found by name in the global [`Fonts/` registry](../formats/text/fonts.md).

## Pipeline in OPTS

1. `OftFile::parse` reads every `.oft` below `Fonts/` once (1,055 files, 1,127 font definitions, 84,013 glyph cells; none fail). Lines are matched only at column 0, because the files embed an indented tutorial that mentions `[newfont]` and `[char]`.
2. `FontRegistry` maps lower-case font name → font. Same name twice: the first file in path order wins (Q-FN01 stays open). Colour and alpha bitmaps are decoded once per font; the alpha bitmap's red channel is the coverage.
3. `resolve_text` picks the string (by number; by name only with a name list, see limits).
4. `render_rgba` lays the string out on one line: each glyph is the cell `[x_start, x_end) × [y_top, y_top + line_height)`, advance = cell width (the cell already includes the gap). Text wider or taller than the texture is shrunk uniformly (never enlarged); it is aligned by orientation horizontally and centred vertically. Mono: text colour from the `.sco`, alpha from the alpha bitmap. Original colours: the colour bitmap's pixels. Background stays transparent.
5. The result is an `Image` (RGBA8, **repeating sampler** — mesh UVs of text fields are often negative; since Phase 8 the shared filtered sampler of [`texture_filter`](texture-quality.md), with a generated mip chain whose colours are averaged weighted by alpha) cached per (font, text, size, colour, orientation). `TextureOverride::image` hands it to the model loader, which uses it instead of the material's file texture.

## Corpus findings that shaped the rules

- Street-name signs use `StreetBerlin`, 256 × 32, black mono; the font's line height (29) fits the texture, so text is drawn at native size.
- Stop objects use the placeholder font name `test` (defined by `Fonts/test1.oft`): the real appearance comes from the string, mono flag and the plate's own colours.
- The `Textfeld_1` plate maps only the middle of its 256 × 128 texture with negative V; with clamping the text was invisible, with wrapping it shows.

## Limits and open questions

| Item | State |
| :--- | :--- |
| Named sources (`[texttexture_enh]` with a variable name, e.g. `Displaytext`) | Not resolved: the object's `[stringvarnamelist]` files are not read yet; logged as an error per object. |
| Runtime references (`\S:n`, `Textfeld_*`) and script-driven text (`STTextOut`) | Wait for the script VM (M-0026). |
| Exact OMSI alignment when text overflows (shrink vs clip) and vertical placement | Unverified ❓; shrink-to-fit and centring chosen. Compare with an OMSI screenshot to confirm. |
| Grid flag of `[texttexture_enh]` | Ignored. |
| Vehicles (`model.cfg` `[texttexture]`) | **Wired 2026-10-12** (`opts-game` `omsi_world/vehicle_text.rs`): `[useTextTexture] n` binds the n-th declaration to the mesh; each declaration owns one image (created blank, no placeholder file is loaded, so the `Mtx_*.bmp` "texture not found" errors are gone) and a `VehicleTextTexture` entity that reads the declaration's **string variable** (`(S.$.Mtx_Linie)`) from the owner's script engine and redraws the image when the text changes (at most 8 redraws per frame). Fonts come from the same registry as the scenery. Empty strings give a transparent image. |
| Content of the vehicle strings | The scripts fill them from the timetable, termini and bus stops through host callbacks (`GetTerminusString`, `GetTTLineString`, …) that OPTS does not implement yet ([M-0093](../roadmap/phase-4/M-0093.md)); `number`, `ident`, `yard`, `SetLineTo` are predefined strings the game must set. Until then displays stay blank; strings that scripts fill themselves (clock, gear, temperature of the NLC cockpit) draw. |
| Script textures (`[scripttexture]`, `ST*` callbacks) | [M-0094](../roadmap/phase-4/M-0094.md). |

## Tests

`opts_core::formats::oft` (parse), `opts_bevy::text_texture` (layout, alignment, colour modes, shrink, string resolution), `opts-core/tests/world_parse.rs` (binding to the preceding material). Visual check: `cargo run -p opts-bevy --example render_text_texture -- <OMSI dir> <font> <text> <w> <h> <out.png>`; in game on Grundorf, the stop plate "Bf. Nordspitze" shows its text.
