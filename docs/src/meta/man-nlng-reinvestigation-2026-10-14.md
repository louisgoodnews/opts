# MAN NL/NG reinvestigation (2026-10-14)

| Metadata | Value |
| :--- | :--- |
| **Task** | Reinvestigate the pink/purple glyphs and wrong-scale panels of `MAN_NL` and `MAN_NG272 - GN92` after the UV decoder work (P-1012, P-1013) |
| **Branch** | `research/man-nlng-reinvestigation` |
| **Method** | OPTS release build of master `f9a1237` (+ a throw-away diagnostic patch, Appendix A, never committed); no OMSI binary was inspected, no OMSI file was modified |
| **Reference vehicle** | `MAN_SD200/MAN_SD83.bus` (renders correctly) |
| **Map** | Aachen Linie 33, camera by control protocol (`camera.set_mode`, `camera.look`, `screenshot`) |
| **Earlier documents** | [P-1012 diagnostic](bugs/man-nlng-materials-2026-10-12.md), [UV decoder](../architecture/uv-decoder.md), [NG272 incident files](uv-incidents-ng272-2026-10-13.md), [M-0079](../roadmap/phase-10/M-0079.md) |

## 1. Verdict in five lines

1. The artifacts **still exist** and are the ones the earlier reports describe (section 2).
2. The decoder **has not changed since the deferral**; the exact-match rate is the documented **85.14 %** (section 3). Its improvement from 77 % happened *before* the deferral, on 2026-10-12.
3. The **pink keypad keys are not an artifact**: the panel atlas really contains them (section 2.2). They are removed from the list of defects.
4. The **decoder is a cause but not the whole story as measured**: panel meshes visibly depend on their unreliable UVs; the steering-wheel hexagon comes from `Recovered`/`Unreliable` vertices; the roof glyph patches survive when every `Unreliable` vertex is neutralised, so some of them come from vertices the decoder rates `Exact` or `Recovered`, or from a mechanism no experiment here isolated (section 4).
5. There is **no ground truth for the broken meshes** (no plain twin exists for them), so the remaining question cannot be settled by more screenshots. It needs a new measurement; section 6 proposes it as [M-0104](../roadmap/phase-10/M-0104.md) and the [design](../architecture/uv-decoder-texture-evidence.md). Nothing was implemented.

## 2. Phase A - reproduction

The renders (OPTS output at 1280 px, same map, same start position, `vivid` look) show OMSI textures and are **not committed** (decision of the project lead, 2026-10-14). The tables name the view; Appendix B reproduces every one with the control protocol.

### 2.1 Views of the NG272 (GN92)

| View | Image | What is visible |
| :--- | :--- | :--- |
| Cockpit, looking down | (render not committed) | Dashboard and gauges are coherent. A **pink hexagon** sits on top of the wheel rim. Pink and orange keys on the IBIS keypad. |
| Interior level | (render not committed) | Perforated headliner, handrails, ceiling lamps: correct. The roof structure above the headliner shows **horizontal dark/light stripes**. |
| Interior, up | (render not committed) | The stripes fill the view: a texture sampled along a line (collapsed UVs). |
| Exterior, rear/side | (render not committed) | The roof of the front unit carries **dark hexagon and bar glyphs** that belong to no body texture; the rear unit's roof is clean. |

### 2.2 Reference vehicle (SD83)

| View | Image |
| :--- | :--- |
| Cockpit | (render not committed) |
| Ceiling | (render not committed) |
| Exterior | (render not committed) |

No smeared, mis-scaled or mis-coloured region. The load log has **6** unreliable-UV warnings (NG272: **191**).

### 2.3 Which "artifacts" are real

I converted the two atlases to PNG for reading only (nothing was copied into the repository):

- `GN92_Panel.bmp` (1024x1024) contains the IBIS keypad with **magenta and orange keys**, labelled caps and a MAN-logo steering wheel hub. So the coloured IBIS keys and the hub logo in the render are correct content. This matches the original report's remark that pink glyph patches on the dashboard are atlas labels.
- `GN92_1.tga` (2048x2048, body) contains **no hexagon or bar glyphs** at all (side views, curtain, wheels, lamps, red seat fabric on a yellow field). The glyphs on the roof therefore cannot be correct sampling of that atlas; they are wrong.

| Symptom | Real defect? |
| :--- | :--- |
| Pink/orange IBIS keys | **No**, genuine texture content |
| MAN logo on the wheel hub | Correct |
| Pink hexagon on the wheel rim | **Yes** |
| Roof glyph patches (front unit) | **Yes** |
| Striped roof structure seen from inside | **Yes** |
| Open rear of the second unit in the exterior view | Not examined (probably unrelated to UVs; needs its own check) |

**Do the artifacts match the earlier documentation?** Yes in kind (wrong UV region on the wheel and panels, wrong scale elsewhere). The size is smaller: the wheel's UV outliers are fewer than reported (section 3.3).

## 3. Phase B - decoder status

### 3.1 Has `uvfix` changed since the deferral?

No. `git log` of `crates/opts-omsi/src/formats/o3d/uvfix.rs`: last substantive change `59cbefb` (2026-10-12, lattice and smooth stages, 77.1 % -> 85.1 %). The 2026-10-13 commits only added the quality classification (`8c362f9`), the incident files and cache v8 (`33506a8`), formatting and merges. M-0079 was deferred on 2026-10-13 (`9818939`).

### 3.2 Exact-match rate (re-measured today)

`uv_truth` (about 6 minutes) and `uv_step_eval` (1 min 43 s on 16 threads), over the whole installation:

| Measure | Value |
| :--- | ---: |
| `.o3d` files scanned | 172,541 |
| Candidate twin pairs | 2,595 |
| Vertices evaluated | 881,573 |
| **Exact (UV step, true positions)** | **85.14 %** |
| Meshes whose vertex count is a multiple of 5 | 69.46 % |
| Other meshes | 88.48 % |
| Vehicle meshes only (1,524 pairs, 369,927 vertices) | 84.28 % |

| Stage | Share of vertices | Exact |
| :--- | ---: | ---: |
| plane | 22.3 % | 96.7 % |
| lattice | 18.7 % | 93.3 % |
| pool | 13.3 % | 93.1 % |
| hinge | 0.7 % | 87.2 % |
| smooth | 45.0 % | 73.6 % |

Identical to the documented figures (85.14 % UV step). The "77 %" in the first diagnosis is the pre-P-1013 rate.

**The requested NL/NG twin pair does not exist.** The only five pairs under `MAN_NL_NG` are `Model/Generic` signs (4-40 vertices, 140 in total); all have key `0x0`, were not decoded and give no rate. The meshes that are visibly wrong (wheel, body) have no other file of the same name in the installation (incident file: "other files with this name: 0"). The best available proxy is the vehicle-mesh rate above, 84.28 %.

### 3.3 NG272 itself (load log, current decoder)

| Measure | Value |
| :--- | ---: |
| Meshes flagged (more than 10 % unreliable) | 191 |
| Vertices in the flagged meshes: Exact / Recovered / Unreliable | 7,916 / 22,286 / 25,128 (45.4 % unreliable) |
| Vehicle total (existing incident report) | 22,356 of 47,405 (47 %) |
| `GN92_Lenkrad` (wheel) | 2,263 vertices: 66 Exact, 1,115 Recovered, 1,082 Unreliable (47.8 %) |
| `GN92_main_wagenkasten` (body shell) | 28 % unreliable |

Wheel UV outliers (distance from the median UV greater than 0.2): **213 of 2,263 (9.4 %)** in the original report, **43 of 2,263 (1.9 %)** with today's decoder (`scrambled_uv_report`). That is real progress since the first diagnosis; the decoded range still spans v from -0.887 to +0.159, wider than a rim patch should, so wrong vertices remain. Of the 1,082 unreliable wheel vertices, 1,068 were settled by the smooth stage with an energy margin below 2 ("several offsets explained the neighbours about equally well"); 14 by the hinge stage.

## 4. Phase C - what causes the remaining artifacts

Question: are the remaining artifacts caused by the vertices the decoder is unsure about? No ground truth exists, so I used interventions: replace the UVs of a class of vertices and see which artifacts move. The patch is in Appendix A (`OPTS_UV_EXPERIMENT`, diagnostic only, reverted). Every run used the same vehicle, map and camera script.

| Run | UVs replaced | Cockpit | Exterior |
| :--- | :--- | :--- | :--- |
| `base` | none | (render not committed) | (render not committed) |
| `median` | every **Unreliable** vertex gets the median UV of the mesh's trusted vertices | (render not committed) | (render not committed) |
| `median1` | every **Unreliable and Recovered** vertex gets that median (only Exact kept) | (render not committed) | (render not committed) |

Observations:

1. **`median` makes the dashboard much worse** (garbled gauges and switches), while `base` is mostly coherent. So the best guesses on the panel meshes are largely right; the 47 % "unreliable" label over-states the damage there. It is also a control: replacing the decoder's guesses is visible, so the experiment can see UV changes.
2. **The pink wheel hexagon is gone in `median1`** and present in `base` and `median`: it comes from `Recovered` vertices (90-99 % precision class), not from `Unreliable` ones.
3. **The roof glyph patches are unchanged in `median`** and still present, reduced, in `median1`: they do not come from `Unreliable` vertices, and some survive even when only `Exact` vertices are kept. Caveat: the fill UV of a replaced vertex is itself a point in the atlas and can paint the neighbouring triangle; `median1` therefore cannot prove that Exact vertices are wrong. What it does show is that no *Unreliable-only* explanation holds for the roof.
4. A fourth run (every decoded UV set to (0, 0)) removed all glyphs, as it must for a constant texture coordinate; it carries **no** information about the cause and is not used in the argument.

Conclusion (what is and is not established):

| Claim | Status |
| :--- | :--- |
| UV decoding errors contribute to the wheel hexagon | **Established** (it disappears when `Recovered` vertices are replaced) |
| Unreliable vertices alone cause the roof glyphs | **Refuted** (unchanged in `median`) |
| `Exact` vertices cause the roof glyphs | **Not established** (confounded by the fill) |
| Non-UV cause of the roof glyphs (material or texture selection, repaint, a mesh read as the wrong model) | **Not excluded**; the log shows no missing texture and no repaint line, so there is no evidence either way |
| The decoder's quality labels are calibrated for this vehicle | **Unknown**; calibrated on 1,524 vehicle pairs at 84.28 % but not on any NG272 mesh |

So the category is the third one in the task ("something else, report the evidence"), narrowed: the decoder is *a* cause, the pink-key colour was a false alarm, and the roof glyphs have an unidentified cause that no current tool can separate.

## 5. Why no more screenshots will settle it

The twin truth set measures the decoder where a plain twin exists. The NG272's defective meshes have none, and the decoder cannot be graded on them. Two routes can produce evidence:

1. **Independent evidence in the data we do have** - the texture. The atlas is plain (not scrambled). A correct UV assignment gives texture continuity along triangle edges; a wrong offset lands on a different part of the atlas. This evidence is unused by the decoder today and can be measured offline on the truth set (does it separate the true offset from the others?).
2. **Reconstruct the scrambler's chain** (M-0079 "open ideas"), which would give exact UVs everywhere. The 881,573 labelled vertices are the training material; earlier attempts did not reproduce the chain.

Route 1 is cheap to test and independent of route 2; it is the proposed first step.

## 6. Proposal

- **M-0104 - UV Decoder Texture Evidence** ([milestone](../roadmap/phase-10/M-0104.md), [design](../architecture/uv-decoder-texture-evidence.md)). Phase 1 is a feasibility measurement only; the milestone stops there if it fails.
- M-0079 stays deferred; its fleet report would be the acceptance instrument for M-0104.
- Not part of this task and not done: any change to `uvfix.rs`, to the cache, or to rendering.

## 7. Follow-ups found on the way

- The exterior view of the NG272 shows the second unit open at the rear (see 2.3). Check whether the trailing unit's rear panel is missing or culled.
- The vehicle-mesh pairs under `MAN_NL_NG` that `uv_truth` reports have key `0x0` and `decoded=false`; they are listed as "pairs" but carry no information. `uv_truth` could skip them.
- The warning text says "(best guess used)"; for `Recovered` vertices (up to 10 % wrong) no warning is issued at all, yet they caused the wheel hexagon. Worth a per-vehicle expected-wrong-vertex count (`0.05 x Recovered + 0.29 x Unreliable`) in the log line.

## Appendix A - diagnostic patch (not committed)

`crates/opts-bevy/src/descramble_cache.rs`, in `apply`, before the per-vertex copy:

```rust
let uvs = &experiment_uvs(uvs, quality);

fn experiment_uvs(uvs: &[[f32; 2]], quality: &[u8]) -> Vec<[f32; 2]> {
    let Ok(mode) = std::env::var("OPTS_UV_EXPERIMENT") else { return uvs.to_vec(); };
    if mode == "all" { return vec![[0.0, 0.0]; uvs.len()]; }
    let threshold = if mode == "median1" { 1 } else { 2 };   // 2: Unreliable only; 1: also Recovered
    let mut trusted: Vec<[f32; 2]> = uvs.iter().zip(quality)
        .filter(|(_, q)| **q < threshold).map(|(u, _)| *u).collect();
    let med = |k: usize, v: &mut Vec<[f32; 2]>| {
        v.sort_by(|a, b| a[k].total_cmp(&b[k]));
        v.get(v.len() / 2).map_or(0.0, |x| x[k])
    };
    let fill = if trusted.is_empty() { [0.0, 0.0] } else { [med(0, &mut trusted), med(1, &mut trusted)] };
    uvs.iter().zip(quality).map(|(u, q)| if *q >= threshold { fill } else { *u }).collect()
}
```

The decode cache (`quality` per vertex, cache v8) already holds everything the patch needs.

## Appendix B - reproduction

```text
cargo build --release -p opts-game -p opts-cli
opts-game --windowed --omsi-path <OMSI 2> --map-path "<OMSI 2>/maps/Aachen Linie 33" \
          --vehicle-path <OMSI 2>/Vehicles/MAN_NL_NG/MAN_GN92_main.bus --autostart --traffic 0 --pedestrians 0 \
          --camera cockpit --cli-port N
opts-cli --port N camera.look --dy 220 ; screenshot --name <n>          # cockpit, looking down
opts-cli --port N camera.set_mode pax ; camera.look --dy -250 / 120     # interior
opts-cli --port N camera.set_mode external ; screenshot --name <n>      # exterior
cargo run --release -p opts-core --example uv_truth <OMSI 2> <out dir>
cargo run --release -p opts-core --example uv_step_eval <out dir>/pairs.tsv
cargo run --release -p opts-core --example scrambled_uv_report <OMSI 2>/Vehicles/MAN_NL_NG/Model/model_GN92_main.cfg Lenkrad
```
