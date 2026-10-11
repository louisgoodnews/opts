# Files, Writers and Fidelity

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Goal** | The editor writes files that OMSI 2 and OPTS read, and it changes nothing it was not asked to change |

## 1. Encodings and text conventions

| Aspect | Observed | Editor rule |
| :--- | :--- | :--- |
| Character set | CP1252 (most text), UTF-8, **UTF-16 LE with BOM** (`global.cfg` of *Oberstedt 2* starts `FF FE`), mixed per file | Detect on open (BOM, then strict UTF-8, then CP1252); write back in the same encoding. New files: CP1252 for compatibility with older readers; characters outside CP1252 trigger *UTF-16 with BOM* for files OMSI reads as UTF-16 (`global.cfg`, `.dsc`) or a validation error for the others. |
| Line endings | CRLF in files made on Windows; some LF | Keep the file's style; new files CRLF. |
| Final newline, blank lines, banner comments (`-----`, `*****`, `=====`) | free | Kept verbatim (verbatim runs). |
| Keywords | `[keyword]` alone on a line; case-insensitive in the readers (`[splineAttachement]` vs `[splineattachement]`) | Preserve the spelling found; new blocks use the spelling the specification shows. |
| Parameters | one per line, comments allowed after the parameters | Parameter lines are replaced only for modified blocks; trailing comment lines (`Object Nr. 35`) stay. |
| Numbers | decimal point, 6–15 digits, sometimes comma in hand-edited files | Read tolerant (`,` accepted); write `f32` shortest round-trip, no exponent for common ranges. |
| Paths | backslashes, relative to the OMSI root or the file, case-insensitive | Write backslashes and the case of the *actual file name* in the project or install; never an absolute path. |
| Tool stamps | "Created with OMSI-Editor on …", "Created with Omsi Version 2.2.032" | Kept; the editor does **not** claim to be OMSI — it appends one comment line `Edited with OPTS Editor <version> on <date>` only when the file has a header comment block (configurable). |

## 2. Writers by format

| Format | Writer | Notes |
| :--- | :--- | :--- |
| `.map` (tile) | blocks re-serialised for touched elements; new tile from a template | version 14 layout ([`tiles.md`](../formats/map/tiles.md)); `[object]` with `N` string variables, `[spline]` 18 lines, `[spline_h]` 19, `[splineAttachement]` 12–15, `[splineAttachement_repeater]` 16–20, `[rule]`, `[attachObj]`, `[varparent]`, markers `[terrain]`, `[variable_terrain]`, `[variable_terrainlightmap]`, `[water]` |
| `.map.terrain` | binary writer (`u32 60`, `f32[3721]`) | exact 14,888 bytes |
| `.map.water` | binary writer (`u32 1`, `f32[4]`) | 20 bytes |
| `.map.LM.bmp` | 24-bit 256² BMP writer | baked by the editor |
| `.map.prt` | writer for the known layer table; unknown parts kept | |
| `global.cfg`, `.dsc` | text writer (UTF-16 aware) | keeps `[mapcam]`, flags |
| `.sco`, `.sli` | text writer | mesh/material state model preserved (keywords apply to the most recent `[mesh]`) |
| `.o3d` | binary writer, **plain only** | [§3](#3-binary-meshes) |
| `.bus`, `.ovh`, `model.cfg`, `paths.cfg`, `passengercabin.cfg`, `sound.cfg`, `.hum`, `.cti`, `.hof`, `.org` | text writers | `[mass]` 4 lines, `[boundingbox]` 12 lines (both boxes kept), `[newanim]` order |
| `.osc`, `varnamelist`, `constfile` | text, script editor | content is the user's source; the VM validates |
| `ailists.cfg`, `unsched_*.txt`, `parklist*`, lists | text writers | `[end]` closes groups; weights optional |
| `.ttl`, `.ttp`, `.ttr`, `Busstops.cfg`, `StnLinks.cfg`, `Chrono.cfg`, `timeline.prt.cfg` | text writers | header block ("Time Table Trip File …"), index comments (`0:`) regenerated |
| `.dds`, `.tga`, `.bmp`, `.png`, `.jpg` | image writers | DDS with mips; TGA 24/32 bit with a consistent descriptor |
| `.wav` | copy / re-encode | |

## 3. Binary meshes

Writing is limited to **plain** `.o3d`: key `0xFFFF_FFFF`, header flags without the "alternative seed" bit, positions stored as authored. No code path writes scrambled data. A mesh that was *read through the decoder* (OPTS's reconstruction of a scrambled file) is marked `decoded` and cannot be saved, copied into another mesh, or exported (the reconstruction is not the author's data and the author protected it); the **Save** command is disabled for it with the explanation.

Matrix chunk: the writer emits an identity `0x79` chunk or omits it; imported pivots are baked into the vertices (OPTS does not apply the matrix chunk, [Addendum 4](../formats/binary/o3d.md)). Bone weights `0x54` only for characters.

## 4. Safe writing

* **Atomic:** write to `<name>.tmp` in the same folder, fsync, rename over the target.
* **Backups:** the previous version is kept as `<name>.bak` (like OMSI's editor) in the project's `.opts-editor/backups/<timestamp>/` — never next to files in the OMSI install.
* **No partial states:** *Save all* writes in dependency order (textures, meshes, objects, tiles, manifest) and, on a failure, reports exactly what was written; the journal allows resuming.
* **Case-insensitive collisions:** two files that differ only by case are refused on case-insensitive targets and warned on others (OMSI is Windows-first).

## 5. Round-trip guarantee and how it is tested

**Guarantee R1 (text, untouched).** For every installed file of a text format, `print(parse(bytes)) == bytes`.
**R2 (text, touched).** Changing one parameter changes only the lines of that block; all other bytes stay.
**R3 (binary).** For every installed file of a binary format with a writer, `parse(write(parse(bytes))) == parse(bytes)` (semantic equality; byte equality where the format has no padding or ordering freedom — terrain, water, plain `.o3d` with the same chunk order).
**R4 (semantic).** An edited project loads in OPTS and in OMSI 2 with the intended effect; R4 is checked by the loader tests (counts, bounding boxes, screenshots) and by the manual OMSI acceptance checklist per writer.

The corpus test (`cargo test -p opts-core --test roundtrip_corpus -- --ignored`) runs over `$OMSI_PATH` and reports per format: files, failures, first differing byte with the block it belongs to. It becomes part of CI for releases with access to an installation.

## 6. Unknown and open fields

Open fields (marked ❓ in the specifications) are modelled as `Raw(String)` in the typed layer, shown in the inspector as read-only text with a *Treat as number* option for exploration, and written back unchanged. A *Field research* mode records the values of an open field across the project to help specification work (statistics, no writes).

## 7. Version handling

* Tile `[version]` 14 is the layout the editor writes. Files at version 10 (older tiles, [`tiles.md`](../formats/map/tiles.md)) open read/write at their own layout (object blocks without pitch/roll); *Upgrade to 14* is an explicit command with a diff.
* `.o3d` versions 1, 4, 5, 7 are read; new files are 7.
* Format features introduced by later OMSI patches (`_2` keyword variants such as `[aigroup_2]`, `[path_2]`, `[light_enh_2]`) are written only when the file already uses them or the user enables the feature; the inspector shows a *Requires OMSI ≥ …* hint when the specification knows it.
