# UV Decoder: Texture Evidence (design)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft - proposal, nothing implemented |
| **Milestone** | [M-0104](../roadmap/phase-10/M-0104.md) |
| **Extends** | [uv-decoder.md](uv-decoder.md) (pipeline, `UvQuality`) |
| **Motivation** | [MAN NL/NG reinvestigation 2026-10-14](../meta/man-nlng-reinvestigation-2026-10-14.md) |
| **Clean room** | Uses only plain files already read by OPTS (meshes, textures); no executable is inspected |

## Problem

Meshes without a plain twin are decoded from geometry alone. The smooth stage settles 45 % of all vertices at 73.6 % exact and, on the NG272, leaves about 47 % of the vehicle's vertices `Unreliable`; `Recovered` vertices (90-99 % right) still produced a visible defect (the wheel hexagon). Geometry is the only evidence the decoder uses. Whatever else is available for the same mesh is unused.

## Idea

For every vertex the scrambler's offset comes from a small set (79 values). The candidate UVs of one vertex therefore point to up to 79 places in the texture atlas. The atlas is **not scrambled** and the true UVs of neighbouring vertices sample texels that belong together: along a triangle edge the sampled colour changes smoothly inside a texture island and jumps only at island borders. A wrong offset puts the vertex in another part of the atlas, and the edges to its correctly decoded neighbours then cross unrelated texels.

This gives a second energy term for the smooth stage, independent of the geometric one:

```text
E_tex(v, o) = sum over edges (v, w) of  d( T(uv_v + o), T(uv_w) )     (w settled or candidate)
```

`T` is the atlas sampled at the (wrapped) coordinate; `d` is a colour distance over a short run of samples along the edge, not just the endpoints, so that high-contrast but legitimate texture detail (labels, gauges) does not dominate.

## Constraints and risks (stated before measuring)

- Atlas content that is flat (body paint) gives no evidence; strongly textured parts (dashboard) give a lot. The term must be weighted by local texture variance and must vanish on flat regions.
- Atlas islands are surrounded by padding; edges that legitimately cross islands (seams) are penalised wrongly. The term must be a likelihood among the 79 candidates, not an absolute penalty, and be learned or calibrated on the truth set.
- Repaints replace the texture; the evidence uses whichever texture the material resolves to. If it cannot be resolved, the stage is skipped (no regression).
- Material and texture resolution is outside the mesh decoder today; the decoder needs a narrow interface `TextureProbe { sample(material, u, v) -> [f32; 3] }`, implemented in `opts-bevy` (it already loads textures), and a null probe in `opts-core` for tests.
- Cache keys must include the texture identity (the decode depends on it), cache version 9.
- Determinism: sampling must use the CPU image data with fixed filtering (nearest or bilinear, defined), not the GPU.

## Phases

### Phase 1 - feasibility (offline, `opts-core` example, no behaviour change)

Question: on the truth set, does the texture term separate the true offset from the wrong ones, and where?

- Input: `pairs.tsv`/`vertices.tsv` from `uv_truth` (true UVs known) plus the materials/textures of the plain twin.
- For each vertex with known true offset, compute `E_tex` for all 79 candidates; report rank of the true offset, top-1 and top-3 rate, by stage and by local texture variance.
- Go/no-go: the term must rank the true offset first for at least 60 % of the vertices the smooth stage settled with margin below 2 (the stage's failing group), and must not rank it worse than random for any large family (vehicles, scenery, humans).
- Output: `docs/src/meta/uv-texture-evidence-feasibility-YYYY-MM-DD.md` with the numbers; the milestone stops here on a no-go.

### Phase 2 - integration (only after a go)

- Add `E_tex` to the smooth stage's energy with a weight set by Phase 1.
- `TextureProbe` interface; implementation in `opts-bevy`.
- Recalibrate `UvQuality` thresholds on the truth set (the thresholds 99 % / 90 % are defined by measured precision and must be re-measured, since the evidence changes).
- Cache v9, tests in `crates/opts-core/tests/uv_decoder.rs` using a synthetic atlas fixture (self-authored).

### Phase 3 - acceptance

- Truth-set exact rate (UV step) improves from 85.14 % by at least 3 points; no family regresses by more than 0.5 points.
- NG272: the wheel hexagon and the roof glyph patches are gone or reclassified, shown with the same screenshot script as the reinvestigation. If the roof glyphs persist, they are not a UV-decoder problem (record it and open a material/texture-selection investigation).
- Per-vehicle log line reports expected wrong vertices, not only the `Unreliable` count (third follow-up of the reinvestigation).

## Alternatives

| Alternative | Why not first |
| :--- | :--- |
| Reconstruct the scrambler chain (M-0079 "open ideas") | Would give exact UVs everywhere, but earlier attempts did not reproduce it and it is open-ended research; Phase 1 above costs a day and is independent of it. Both can run. |
| Hinge-interpolated fallback | Measured worse than the best guess (M-0079). |
| Raise the quality thresholds / hide unreliable meshes | Hides the problem; does not recover UVs. |
| Ask the user to repaint or supply plain files | Not applicable to an installation we must not modify. |

## Open questions

1. How are materials resolved for meshes with several materials per mesh (face materials)? Phase 1 can restrict to single-material meshes first.
2. Mip level for sampling: finest level, to avoid blurring island borders.
3. Does the same evidence help scenery and humans (large share of the truth set)? Phase 1 reports by family so this is answered by data.
