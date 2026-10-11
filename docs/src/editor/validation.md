# Validation

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Specification |
| **Engine** | `opts-edit::validate`, incremental; also used headless by `opts validate` |

## 1. Model

A **rule** inspects one element, one document or the whole project and returns *findings*: `{ rule id, severity, document, element, line, message, specification link, fix? }`.

| Severity | Meaning | Effect |
| :--- | :--- | :--- |
| **Error** | OMSI 2 or OPTS cannot load it, or the result is certainly wrong | blocks *Export package*; *Play from here* warns |
| **Warning** | loads but is likely unintended or fragile | listed; export allowed |
| **Info** | style, performance, portability hints | optional |
| **Note** | the specification marks the situation as open ❓ | shown once per project, never counted |

Rules run **incrementally**: a change event re-runs the rules that watch the changed element kinds and cross-file rules whose inputs changed. Findings carry a stable key so they do not flicker. Rules are plain functions registered in a table, each with tests (a minimal failing input, a passing input, the corpus run showing the false-positive rate on the installed content).

## 2. Rule catalogue (initial set)

Ids are `<area>-<nnn>`; the table is the contract for the first implementation, not a limit.

### Syntax and references

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| REF-001 | E | Referenced file does not exist at its path | *Use relocated file* when exactly one same-named file exists or the folder match is clear ([relocation](../formats/binary/o3d.md)); else *Pick file* |
| REF-002 | W | Referenced file found only under another folder name (works in OPTS via relocation, **not** in OMSI) | *Rewrite reference* |
| REF-003 | E | Absolute or drive-letter path | rewrite relative |
| REF-004 | W | Case differs from the file on disk | normalise case |
| REF-005 | E | Texture format unsupported or header invalid | convert (e.g. TGA descriptor repair) |
| SYN-001 | E | Keyword with too few parameters | open source view at the line |
| SYN-002 | W | Unknown keyword (kept) | — |
| SYN-003 | W | Number unparsable or out of the corpus range | clamp suggestion |
| ENC-001 | E | Character outside the file's encoding | switch to UTF-16 where allowed |

### Maps

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| MAP-001 | E | Tile listed in `global.cfg` is missing | create empty tile / remove entry |
| MAP-002 | E | Duplicate ID code | renumber |
| MAP-003 | W | `[NextIDCode]` ≤ highest used ID | bump |
| MAP-004 | W | Spline connection ends farther apart than 5 cm | snap |
| MAP-005 | W | Spline end touches nothing and is not marked dangling | connect / mark |
| MAP-006 | E | `[splineAttachement]` names a spline the tile lacks | re-attach / delete |
| MAP-007 | W | Object far below or above the terrain (> 50 m) | snap to terrain |
| MAP-008 | W | Terrain border heights differ > 1 cm between neighbours | average |
| MAP-009 | W | Terrain not carved under a spline with `[heightprofile]` (road under ground) | bake carve |
| MAP-010 | E | `[entrypoints]` record without its object, or object without record | create |
| MAP-011 | W | Road component not reachable from any entry point | highlight |
| MAP-012 | I | Tile vertex budget above the configured limit | list heaviest |
| MAP-013 | N | Chrono tile replaces which parts of the base tile ❓ | — |

### Traffic and signals

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| TRF-001 | W | Lane crossing without priority rule or signal | suggest |
| TRF-002 | E | `[use_traffic_light]` index out of range | clamp |
| TRF-003 | W | Signal group without phase covering a lane | add phase |
| TRF-004 | W | Path end connected to nothing (road, pedestrian or rail) | highlight |
| TRF-005 | W | Speed limit missing on a road class that normally has one | suggest |
| TRF-006 | E | `humans.txt`/`drivers.txt`/`parklist_p.txt` entry missing or of the wrong type (stops OMSI loading the map) | remove entry |

### Assets and vehicles

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| AST-001 | E | Mesh references material slot without `[matl]` or texture | add |
| AST-002 | W | Texture not a power of two, or larger than 4096 | resize |
| AST-003 | W | Texture alpha present but material has no alpha mode (alpha ignored by OMSI) | strip alpha or set mode |
| AST-004 | E | Protected (scrambled) mesh would be written or exported | blocked |
| AST-005 | W | Mesh pivot not at ground contact / object far from origin | recentre |
| AST-006 | W | Degenerate or flipped triangles above 0.5 % | list |
| AST-007 | I | Triangle count above the class budget (object, vehicle, character) | LOD |
| SCO-001 | W | `[boundingbox]` does not contain the mesh | fit |
| SCO-002 | E | `[visible]`/`[newanim]` uses a variable missing from the variable list | add variable |
| SLI-001 | E | Profile points laid out right to left (bottom face up) | reverse |
| SLI-002 | E | `[path]` type outside 0–3 or width ≤ 0 | fix |
| VEH-001 | E | `[model]` missing / model.cfg without meshes | — |
| VEH-002 | W | `[boundingbox]` extents do not match the body (full-extent convention, [Addendum 9](../formats/binary/o3d.md)) | fit |
| VEH-003 | W | Axle with no wheels or wheel mesh far from its `[newanim]` pivot (> 0.35 m) | move pivot |
| VEH-004 | E | `[CTC]` folder missing or no `.cti` defines the default repaint of `[friendlyname]` | add |
| VEH-005 | W | Script compile diagnostics (`opts_sim::script`) | go to line |
| VEH-006 | W | Script variable used but never written | list |
| HUM-001 | E | Character mesh lacks the 13-bone weight chunk | — |
| HUM-002 | W | `[links]` pivots inconsistent with `[humangeom]` height | rescale |

### Transport

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| TT-001 | E | `[station_typ2]` ID has no bus-stop object / `Busstops.cfg` entry | create entry |
| TT-002 | W | Consecutive stops of a trip not connected by drivable path | show gap |
| TT-003 | E | `.ttl` `[addtrip]` names a `.ttp` that does not exist | — |
| TT-004 | W | Tour trips overlap in time or the end stop is far from the next start | retime |
| TT-005 | W | Running time shorter than free-flow time at the speed limits | lengthen |
| TT-006 | W | Timetable day not covered by any chrono entry | extend |
| TT-007 | E | Depot (`[aigroup_depot]`) `.hof` file missing | — |
| TT-008 | W | AI list vehicle not installed / repaint name unknown to the vehicle | replace |
| TT-009 | W | Duplicate fleet number within a depot | renumber |
| TT-010 | I | Route strings do not fit the destination matrix | shorten |

### Licence and packaging

| Id | Sev. | Check | Fix |
| :--- | :---: | :--- | :--- |
| PKG-001 | E | File to embed in the package is third-party and not confirmed redistributable | confirm / reference |
| PKG-002 | W | Dependency on a pack that is not listed in the manifest | add |
| PKG-003 | W | Package would overwrite a file of the base installation | rename |
| PKG-004 | I | Missing credits or licence text | add |

## 3. Output and CI

* In the editor: panel, gutter markers, outliner badges, tinted viewport elements.
* Headless: `opts validate <project|folder> --format json|text|sarif --fail-on error`; exit code 0/1; `--rules` lists the catalogue; `--baseline file` ignores known findings. SARIF output lets code-review tools annotate project repositories.
* **Corpus regression:** `opts validate $OMSI_PATH --only SYN-001,REF-001,…` runs over the installation; the result per rule (counts, false-positive review notes) is stored in `docs/src/meta/` when a rule is introduced, so that a rule that would flag 40 % of the installed content is recognised as a wrong rule, not a content problem.

## 4. Auto-fix policy

Fixes are commands (undoable), preview-able (*show diff*), never silent, and never cross a protected-content boundary. *Fix all* groups by rule and asks once per rule. Heuristic fixes (relocation, priority suggestions, running times) are labelled *suggestion* and never applied by *Fix all*.
