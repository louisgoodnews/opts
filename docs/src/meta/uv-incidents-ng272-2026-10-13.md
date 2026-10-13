# UV incident files of the MAN NG272 - GN92: what the 191 files say

Run: `MAN_GN92_main.bus` on Aachen Linie 33, release build of the merged tree, cold decode cache (every mesh decoded in the background), `--log-file <dir>`. Result: **191 incident files** (2.2 MB) in `<dir>/incidents/`, one per scrambled mesh with more than 10 % unreliable vertices, out of 217 scrambled meshes of the vehicle. Summary tool: `python3 scripts/uv_incident_stats.py <incident dir>`.

This page explains how to read one file, what the whole set shows, and what it means for what you see on screen.

## 1. How to read an incident file
```
mesh / context / header / vertices ...      which file, who loaded it, how big
== Result ==                                vertices per quality: Exact, Recovered, Unreliable
== What settled the vertices ==             per decoder stage: how many vertices, how many unreliable
== Why ==                                   the reasons for the unreliable ones, in numbers
== Unreliable vertices ==                   one row per vertex (first 400):
   index (decoded position) (stored u v) (best-guess u v) du dv  stage evidence
```
- **stored uv**: the value in the `.o3d` file. It is the true texture coordinate plus the scrambler's offset (or just the true one when the scrambler added nothing).
- **best-guess uv**: what the decoder shows on screen. `stored - guess` is the offset it removed (`du`, `dv`). `du +0.0000 dv +0.0000` means: *the decoder decided that the scrambler added nothing to this vertex*.
- **stage / evidence**: which decoder stage settled the vertex and how strong its evidence was. For `smooth` the evidence is the energy margin between the chosen offset and the runner-up: below 2 means the neighbours explain two or more offsets about equally well; the unreliable ones typically sit at 0.5-1.6.
- **Exact / Recovered / Unreliable** are calibrated on the 881,573-vertex twin truth set: 99 % / 90-99 % / below 90 % of such vertices were right there (`docs/src/architecture/uv-decoder.md`).

## 2. What the set shows
| Measure | Value |
| :--- | :--- |
| Meshes flagged | 191 of 217 (the other 26 are fine or skipped) |
| Vertices in the flagged meshes | 55,330: Exact 14.7 %, Recovered 40.1 %, **Unreliable 45.2 %** |
| Unreliable share per mesh | median 75 %, 63 meshes between 90 and 100 %, 20 meshes between 10 and 30 % |
| Decoding source | all "background decode" (cold cache); a second run reads the cache and writes the same files |
| Meshes with another file of the same name in the installation | 41 of 191 (name only: not necessarily the same model) |

**The vehicle total** (log line at spawn): 22,356 of 47,405 vertices (47 %) unreliable.

### By size: many small meshes, but the vertices are in a few large ones
| Mesh size | Meshes | Vertices | Unreliable |
| :--- | ---: | ---: | ---: |
| up to 50 vertices | 122 | 2,410 | 72.1 % |
| 51-200 | 35 | 3,771 | 51.9 % |
| 201-1,000 | 19 | 9,060 | 45.4 % |
| 1,001-5,000 | 15 | 40,089 | 42.9 % |

Two thirds of the incident files are tiny parts (switches, needles, lamps, wipers, mirrors: Thermometer 24 vertices, 96 % unreliable), but they hold only 4 % of the vertices. The weight is in the big meshes: `GN92_main_sitze` (seats, 4,871 vertices, 54 %), `GN92_panel` (3,126, 51 %), `GN92_articul_bellows_ext` (3,831, 41 %), `GN92_main_Stangen` (grab poles, 2,950, 50 %), `GN92_main_innenraum` (3,471, 42 %), `GN92_main_wagenkasten` (body, 4,573, 29 %), `GN92_Lenkrad` (steering wheel, 2,263, 48 %), `Zahltisch+Drucker` (ticket desk, 1,542, 66 %).

### By part family
| Family | Meshes | Vertices | Unreliable |
| :--- | ---: | ---: | ---: |
| `GN92_main_*` | 19 | 25,617 | 40.8 % |
| other `GN92_*` | 28 | 11,424 | 44.0 % |
| `Generic\` shared parts | 95 | 9,791 | 53.3 % |
| cockpit panel parts `GN92_Panel_*` | 39 | 4,230 | 53.2 % |
| doors | 8 | 2,304 | 36.3 % |
| windows | 2 | 1,964 | 61.4 % |

Small, detailed, mostly flat-coloured or atlas-mapped parts fare worst; large smooth bodies (body shell 29 %, doors 36 %) best.

### By decoder stage
| Stage | Vertices | Unreliable among them |
| :--- | ---: | ---: |
| plane | 9,772 | 0 |
| lattice | 5,749 | 0 |
| pool | 6,483 | 0 |
| hinge | 1,290 | 1,290 (all) |
| smooth | 32,036 | 23,697 (74 %) |

Every unreliable vertex comes from the two weakest stages: the **smooth** stage (it settles 58 % of the vertices of these meshes, and three quarters of those stay unreliable) and the **hinge** stage (prediction across a single shared edge). The stages that *prove* an offset (plane, lattice, pool) never produce an unreliable vertex by construction.

## 3. What is going on
1. **The scrambler added an unknown offset (0.0025-0.96 in u or v) to about half of all vertices.** The file does not say which. The decoder has to infer it from geometry: neighbouring vertices of a flat or smoothly mapped surface must lie on a common texture plane.
2. **Where the geometry constrains the answer, it is exact** (plane, lattice, pool: 22,000 vertices of these meshes, 93-97 % right on the twin data). **Where it does not, the decoder guesses.** Tiny meshes have too few vertices to constrain anything; curved or atlas-packed surfaces (seats, panel, wheel, bellows) have no common plane; meshes whose vertex count is a multiple of 5 have almost no jitter-free anchors.
3. **The guess is almost always "nothing was added".** Of the 13,522 unreliable vertices listed in the files, 81.9 % keep their stored value unchanged. That is the prior at work: when several offsets explain the neighbours equally well, "no offset" is the single most likely one (about 50 % of vertices have none). On the twin truth set the unreliable tier has this profile: 85 % of its vertices are left unchanged, only 36 % were really unjittered, and **71 % of the guesses are right**.
4. **The 29 % that are wrong are mostly missed jitter.** Of the wrong guesses on the twin set: 79 % left a jittered vertex jittered, 13 % removed the wrong offset, 8 % shifted a vertex that had no offset. A missed offset moves the vertex's texture coordinate by a typical 0.25 (median; up to 0.96): it samples a different part of the texture atlas. That is the pink patches on the steering wheel (an atlas of dashboard labels), the stripes on the console, the wrong region on a seat.
5. **Nothing is lost in the geometry.** The positions are decoded separately and are right; only the texture lookup of the unreliable vertices is wrong, and only for the share the file reports.

## 4. What the files tell you to do
- **Which meshes matter for the picture:** sort by unreliable *vertices*, not by share: the twelve largest incident files carry 63 % of all unreliable vertices (the seven named above 45 %). Tiny parts at 96 % are invisible in practice.
- **Which ones can be fixed:** a mesh whose file has a twin or copy elsewhere (the "other files with this name" line, and only if it is the same model) is exact when the twin is plain. For the NG272, 150 of 191 meshes have no namesake at all, so there is no data to recover them from.
- **Which could improve with more decoder work:** the large flat or planar meshes with a high share (windows 62 %, body 29 %) would profit from any further constraint; the atlas-packed ones (seats, console, wheel) cannot be proven from geometry alone, which is what the twin data shows.
- **Whether the decoder will ever be exact here:** only if the scrambler's generator is reconstructed (declined for now, see `uv-decoder.md` and M-0079).

## 5. Limits of this analysis
- One vehicle (GN92 main with its trailer, loaded together), one cold-cache session.
- The reasons table of an incident file is the decoder's own view (evidence below a threshold); it does not know the truth. The error profile in section 3 (point 3 and 4) is from the twin truth set, not from the NG272 itself, for which no ground truth exists.
- "Siblings" counts files of the same name; it is an upper bound for twins and copies.
