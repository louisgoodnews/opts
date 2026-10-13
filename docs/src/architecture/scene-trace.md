# Scene Trace (M-0087): which mesh is where

Scope: an opt-in debug mode that keeps one record per placed thing and writes it as JSON Lines
next to the run logs. A defect seen in a screenshot becomes a source file in one call
(`trace.pick`), for people and for agents. Code: `crates/opts-bevy/src/scene_trace/` (records,
store, filters, picking), `crates/opts-game/src/in_game/omsi_world/trace.rs` (the loader hooks),
`crates/opts-game/src/control_trace.rs` (protocol and overlay).

## 1. Modes and cost

| Mode | Keeps | Written to the file |
| :--- | :--- | :--- |
| `off` (default) | nothing | nothing |
| `summary` | counts per tile and kind; a record for every thing with an issue | header, issue records, one `tile_summary` per tile, footer |
| `full` | a record for every placed thing | everything |

Switch it on with the setting *Settings → Advanced → Scene trace* (`advanced.trace_scene`: off,
summary, full; it takes effect for the next map load), with the command line flag
`--trace-scene[=summary|full]` (for one run; the flag wins over the saved setting at start-up), or
through the protocol (`trace.set`). Meshes loaded while the trace was off carry no decode
information: the records say `stage: "cached"` for them. Switch it on **before** loading the map.

**Off is free.** Every hook tests one atomic (`scene_trace::enabled()`); no record, string or
collection is built and the state is not allocated (`off_allocates_nothing`). The loader's
per-file notes (`MeshNote`) are collected on stack variables that stay empty while off (`Lap`
is inactive, no `Instant` is read).

Filters (any combination) decide which things are recorded: tiles, kinds, a file pattern
(substring or `*` wildcards, case-insensitive, matched against the thing's file and its mesh
files) and a radius around a point (Bevy `x`, `z`; the camera when no centre is given).

## 2. The record

One JSON object per thing (`TraceRecord`):

| Field | Meaning |
| :--- | :--- |
| `id`, `t_ms` | running number and milliseconds since the run started |
| `kind` | `scenery`, `tree`, `attachment` (object placed along a spline), `parked` (vehicle model placed as scenery), `spline`, `terrain`, `traffic`, `human`, `vehicle` |
| `tile`, `tile_file`, `list`, `index` | tile `(x, y)`, its `.map` file and the position of the record in that tile (`objects`, `attachments` or `splines`; the index counts in the tile's own list) |
| `file` | source file as the map names it (`.sco`, `.sli`, `.bus`, `.hum`) |
| `pos`, `rot_deg` | position in the tile's OMSI coordinates; heading, pitch, roll as stored |
| `world` | the transform used in the game (Bevy space: translation, quaternion, scale) |
| `bounds` | local bounding box of the geometry (what `trace.pick` tests) |
| `entity` | Bevy entity (bits) of the thing's root |
| `meshes` | the mesh files used (`MeshNote`): `file`, `stage`, `uv` (exact/recovered/unreliable vertices), `vertices`, `parts`, `materials` (slot, texture, `opaque`/`mask`/`blend`, textured), timings `read_ms`/`parse_ms`/`decode_ms`/`build_ms`, `warnings`, `bbox` |
| `parents` | what the thing belongs to: the carrying spline of an attachment, the `carpark` marker a parked car replaced |
| `result` | `ok`, `no-geometry` (nothing to draw by design), `pending` (waits for a background decode; the tile is rebuilt), `failed` |
| `warnings`, `load_ms`, `detail` | problems seen, time spent, kind-specific details (spline length/radius/ids, string variables, vehicle name) |

Mesh `stage` says how the geometry was obtained: `plain` (readable file), `twin` (plain twin of a
keyed mesh), `decode-cache`, `decode-background`, `decoded-now`, `decode-deferred`, `rejected`
(keyed, decoder gave up: nothing drawn, MI-0017), `extent` (implausible size), `read-failed`,
`parse-failed`, `hidden` (all slots hidden by `[alphascale]` overlays), `cached` (served from the
cache of an earlier load, see above).

A record is an *issue* when `result` is `failed`, it has warnings, one of its meshes was rejected
or failed, or more than 10 % of a decoded mesh's vertices have unreliable texture coordinates.
`quality()` condenses this to `failed`, `unreliable`, `decoded`, `ok` (overlay colours).

## 3. The file

`<log directory>/trace/scene-trace-<unix time>-<map>.jsonl`, one file per map load, the ten newest
are kept. Every line has a `type`:

* `header` (first line): `schema` (1), `build` (name, version, profile), `map`, `mode`, `started_unix`, `filters`, `size_cap`;
* `record`: `{type, record}`;
* `tile_summary`: counts per kind, issues per kind and decode stages of one tile (summary and full);
* `tile_reload` (a tile was rebuilt, e.g. after background decoding: earlier records of it are superseded), `tile_unload` (streamed out);
* `truncated` when the size cap (256 MiB; `trace.set size_cap_mb`) stopped the file, `footer` (last line).

Paths are relative to the OMSI folder; a path outside it is reduced to the file name. Nothing
else is recorded: no user names, no absolute paths. The human log gets one line when the trace
file opens and closes.

The in-memory index (for the protocol) keeps up to 150,000 records; older tiles' records leave it
when the tile is streamed out (the file keeps them).

## 4. Control protocol

All except `trace.set` answer `1001 NOT_READY` while the trace is off.

| Method | Purpose |
| :--- | :--- |
| `trace.set` | `mode` (`off`/`summary`/`full`), filters (`tiles`, `kinds`, `file`, `radius` + `center`, `reset`), `size_cap_mb`, `overlay`; answers the run status (file, bytes, counts, filters). Without arguments it is the status query. |
| `trace.pick` | what is under pixel `x`,`y` (physical pixels, as in a `screenshot`) or, without a point, under the crosshair (view centre): the full record of the nearest box on the ray, its parents, further candidates with distances, the ray and the ground point with its tile. |
| `trace.tile` | counts and records of one tile (`tile:[x,y]`, optional `kind`, `file`, `stage`, `issues_only`, `full`, `limit`). |
| `trace.find` | records by `file` pattern, `kind`, decode `stage`, `tile` or `issues_only`. |
| `trace.region` | records within `radius` of a ground point (`x`, `z` in Bevy world coordinates; the camera by default). |
| `trace.export` | writes the matching in-memory records as JSON Lines to an allowed path. |

Picking tests the ray (camera through the pixel, or through the view centre) against the oriented
bounding boxes of the records, nearest entry first; boxes are conservative (a lamp post's box is
bigger than the post), so `candidates` lists the next ones too. Terrain and splines have no box:
the answer's `ground.tile` names the tile and `trace.tile` lists its splines.

## 5. Overlay

*Settings → Advanced → Scene trace overlay* (or `trace.set overlay=true`) draws a wire box around
every traced object within 120 m of the camera: red = failed, orange = unreliable texture
coordinates, yellow = decoded, blue = vehicles, violet = humans, green = scenery; the object of the
last `trace.pick` is white.

## 6. Recipe: screenshot defect to asset reference

1. `opts --trace-scene=full ...` (or `trace.set mode=full` and load the map again).
2. `screenshot` and note the pixel of the defect.
3. `trace.pick x=<px> y=<py>`: `hit.file` is the `.sco`, `hit.meshes[*].file` the `.o3d`, `stage`
   and `uv` say how it was decoded, `hit.tile_file` and `hit.index` locate the record in the map.
4. Look the file up in the [mesh issue registry](../quality/mesh-issue-registry.md) (and, for
   scrambled meshes, in the incident files of the run, `docs/src/architecture/uv-decoder.md`); a new
   defect gets a new `MI-` id.
5. `trace.find file=<name>` shows every other place the same mesh is used.

Related: [mesh issue registry](../quality/mesh-issue-registry.md), [UV decoder](uv-decoder.md),
[control protocol](control-protocol.md), [M-0087](../roadmap/phase-5/M-0087.md).
