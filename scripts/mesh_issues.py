#!/usr/bin/env python3
"""Builds the mesh issue registry (P-1014) from the offline audit output.

Inputs (all produced by examples of `opts-core`, see docs/dev/tasks/P-1014):
  --audit  TSV of `mesh_audit <omsi> --decode`   (path, codes, metrics) per .o3d
  --tiles  TSV of `tile_audit <omsi>`            (map, tile, code, detail)
  --omsi   the install, only used to hash flagged files so identical copies form one entry
  --live   optional directory of live-sweep JSON files (maps and vehicles loaded in the game)

Writes docs/src/quality/mesh-issue-registry.md (classes with counts, evidence, root cause, status)
and mesh-issue-registry.csv (every flagged asset, identical files merged).

Usage: scripts/mesh_issues.py --audit dec.tsv --tiles tiles.tsv --omsi "<OMSI 2>" [--live DIR]
"""
import argparse
import collections
import csv
import hashlib
import json
import os
import re
import sys

# code -> (id, kind, detector description, root cause, status)
CLASSES = {
    "PARSE_ERROR": ("MI-0001", "unreadable file", "`o3d::parse` fails",
                    "zero-length, truncated or foreign file in the source content (a few carry a wrong magic/version on purpose: `OPTS_Test` fixtures)",
                    "content defect (mesh skipped, error logged)"),
    "NONFINITE": ("MI-0005", "non-finite vertex data", "`audit::check` NONFINITE / HUGE_BBOX",
                  "`inf` coordinates in `kfznhalterung.o3d` (five vehicles); would make the mesh bounding box NaN",
                  "fixed (faces with non-finite vertices are skipped in the converter, regression test `finite_tests`)"),
    "EMPTY": ("MI-0006", "empty mesh", "no vertices or faces",
              "placeholder files that hold only the matrix chunk",
              "content defect (nothing to draw)"),
    "MATIDX_OOR": ("MI-0007", "material index out of range", "face material index >= material count",
                   "exporter wrote more material slots than the table holds (e.g. 28 materials)",
                   "content defect (loader falls back to a default material)"),
    "NO_MATERIALS": ("MI-0008", "no material table", "material count 0 with faces",
                     "mesh authored without materials (collision/shadow helpers)",
                     "content defect (default material)"),
    "HUGE_BBOX": ("MI-0009", "huge bounding box", "bbox diagonal > 3000 m",
                  "a few far-away vertices or intentionally huge ground/sky meshes; the loader refuses plain meshes above 3000 m and decoded ones above 400 m",
                  "content defect / tolerance (extent gate)"),
    "HUGE_TRIS": ("MI-0010", "huge triangles", "longest edge > 25x the median edge and > 100 m",
                  "single stretched triangles (light cones, billboards) in plain meshes; in decoded meshes spikes from wrongly labelled vertices",
                  "plain: content defect; decoded: tolerance (spike faces dropped, MI-0018)"),
    "DEGENERATE_TRIS": ("MI-0011", "degenerate triangles", "> 5 % of the triangles have repeated indices or zero area",
                        "exporters that pad quads/strips with repeated indices; harmless to the GPU",
                        "benign (no tolerance needed)"),
    "UV_HUGE": ("MI-0012", "texture coordinates beyond +-1000", "any \\|uv\\| > 1000",
                "tiling textures authored with large UV repeats; scrambled-file jitter is at most 1 unit",
                "benign (wrap sampling)"),
    "UV_CHAOS": ("MI-0013", "texture-coordinate chaos", "UV edge / 3D edge ratio > 40x median on > 10 % of the triangles",
                 "texture density varies legitimately (trim sheets, tiling ground); precision 7 % on decoded twins, so it is **not** a defect indicator",
                 "informational (detector retired as a defect signal)"),
    "WINDING_MIXED": ("MI-0014", "mixed winding", "25-75 % of the triangles disagree with the stored normals",
                      "meshes with inner/under faces or one-sided parts; the converter flips a whole submesh by majority",
                      "informational"),
    "OUTLIER_VERTS": ("MI-0015", "outlier vertices", ">= 3 vertices beyond 8x the 90th percentile radius",
                      "tall thin objects (lamp posts, masts); not corruption",
                      "informational (detector noisy)"),
    "TEX_MISSING": ("MI-0016", "texture not in the install", "texture file name not found anywhere in the install (any extension)",
                    "repaint placeholders, dynamic text textures and files missing from the content pack",
                    "content defect (loader falls back to the material colour)"),
    "DECODE_REJECTED": ("MI-0017", "keyed mesh not decodable", "`decode_confident` rejected (two runs disagree < 0.9, < 4 vertices, > 30000 vertices)",
                        "small or symmetric meshes: the geometric solver cannot tell the four permutations apart",
                        "open (mesh skipped; research problem, see o3d.md Addenda 5-10)"),
    "dec_HUGE_TRIS": ("MI-0018", "decoded mesh with spike faces", "HUGE_TRIS after decoding",
                      "a wrongly labelled vertex of an accepted decode (82 % of accepted twins are exact; 18 % have wrong vertices)",
                      "tolerance (faces longer than 40x the median edge are dropped, `drop_spike_faces`)"),
    "dec_OUTLIER_VERTS": ("MI-0018", "decoded mesh with spike faces", "OUTLIER_VERTS after decoding", "see MI-0018", "see MI-0018"),
    "dec_HUGE_BBOX": ("MI-0018", "decoded mesh with spike faces", "HUGE_BBOX after decoding", "see MI-0018", "see MI-0018"),
    "dec_DEGENERATE_TRIS": ("MI-0011", "degenerate triangles", "after decoding", "see MI-0011", "benign"),
    "dec_UV_CHAOS": ("MI-0013", "texture-coordinate chaos", "after decoding", "see MI-0013", "informational"),
    "dec_UV_HUGE": ("MI-0012", "texture coordinates beyond +-1000", "after decoding", "see MI-0012", "benign"),
    "dec_MATIDX_OOR": ("MI-0007", "material index out of range", "after decoding", "see MI-0007", "content defect"),
    "dec_NONFINITE": ("MI-0005", "non-finite vertex data", "after decoding", "see MI-0005", "see MI-0005"),
    "DECODED_DROPPED_FACES": ("MI-0018", "decoded mesh with spike faces", "`dropped_faces` > 0 in the decoder report",
                              "faces longer than 40x the median edge were removed (tolerance applied)",
                              "tolerance (`drop_spike_faces`)"),
    "DECODED_UV_UNRESOLVED": ("MI-0019", "decoded mesh with unresolved texture jitter", "> 10 % of the vertices match no planar UV model",
                              "the texture-jitter inference (`uvfix`) leaves vertices whose texture coordinates may be off by up to one texture unit",
                              "open (documented limitation, o3d.md Addenda 6-8)"),
}
STATE_ONLY = {"KEYED", "SCRAMBLED_SKIPPED", "DECODED"}

TILE_CLASSES = {
    "TILE_PARSE_ERROR": ("MI-0021", "tile does not parse (61 tiles of 3 maps before the fix)", "`tile_audit`", "version 10 tiles (spline with one connection field, rules without flag, `-1` connections)", "fixed (see table of fixes)"),
    "GLOBAL_PARSE_ERROR": ("MI-0020", "global.cfg does not parse (2 maps before the fix)", "`tile_audit`", "an entry point name starting with `#` was dropped as a comment and shifted every following record", "fixed"),
    "NO_ENTRYPOINTS": ("MI-0022", "map without entry points", "`tile_audit`", "content: map declares none (OPTS falls back to the first road spline)", "content defect"),
    "OBJECT_PLACEMENT": ("MI-0023", "object far outside its tile", "|coordinate| > 5000 m", "editor leftovers: objects dragged far away", "content defect"),
    "SPLINE_PLACEMENT": ("MI-0023", "spline far outside its tile", "|coordinate| > 5000 m", "as above", "content defect"),
    "SPLINE_LENGTH": ("MI-0024", "spline with length 0 or > 2000 m", "`tile_audit`", "zero-length splines produce no mesh", "content defect"),
    "SPLINE_GRADIENT": ("MI-0024", "spline gradient > 100 %", "`tile_audit`", "exotic gradient values (fence/cable splines)", "informational"),
    "SPLINE_RADIUS": ("MI-0025", "spline radius < 1 m", "`tile_audit`", "radius is not what the detector assumes for most of these (mostly valid curves); informational", "informational"),
    "TERRAIN_SPIKE": ("MI-0026", "terrain neighbour step > 50 m", "`tile_audit`", "quarries/cliffs or editor spikes", "content defect"),
    "TERRAIN_NONFINITE": ("MI-0026", "terrain non-finite", "`tile_audit`", "-", "content defect"),
    "TERRAIN_RANGE": ("MI-0026", "terrain height range", "`tile_audit`", "-", "content defect"),
}


def md5_of(path):
    h = hashlib.md5()
    try:
        with open(path, "rb") as f:
            for chunk in iter(lambda: f.read(1 << 20), b""):
                h.update(chunk)
    except OSError:
        return None
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--audit", required=True)
    ap.add_argument("--tiles")
    ap.add_argument("--omsi", required=True)
    ap.add_argument("--live")
    ap.add_argument("--deteval", help="output of mesh_detector_eval (stdout)")
    ap.add_argument("--out", default="docs/src/quality")
    a = ap.parse_args()

    rows = []
    with open(a.audit, errors="replace") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 3:
                rows.append(parts)
    total = len(rows)
    keyed = sum(1 for p, c, m in rows if "KEYED" in c.split(","))
    decoded = sum(1 for p, c, m in rows if "DECODED" in c.split(","))
    rejected = sum(1 for p, c, m in rows if "DECODE_REJECTED" in c.split(","))
    by_top = collections.Counter(p.split("/")[1] if p.startswith("/") and p.count("/") > 1 else "?" for p, c, m in rows)

    # group flagged meshes by (class code, content hash)
    entries = collections.OrderedDict()
    flagged_files = 0
    for path, codes, metrics in rows:
        cl = [c for c in codes.split(",") if c and c not in STATE_ONLY]
        if not cl:
            continue
        flagged_files += 1
        full = a.omsi.rstrip("/") + path
        digest = md5_of(full) or "unreadable:" + path
        for c in cl:
            key = c.split("(")[0]
            if key not in CLASSES:
                key = "PARSE_ERROR" if key.startswith("PARSE_ERROR") else key
            if key not in CLASSES:
                continue
            e = entries.setdefault((key, digest), {"asset": path, "copies": 0, "codes": codes, "metrics": metrics})
            e["copies"] += 1

    per_class = collections.OrderedDict()
    for (key, digest), e in entries.items():
        cid = CLASSES[key][0]
        d = per_class.setdefault(cid, {"keys": set(), "unique": 0, "copies": 0, "examples": [], "meta": CLASSES[key], "codes": set()})
        d["keys"].add(key)
        d["unique"] += 1
        d["copies"] += e["copies"]
        d["codes"].add(key)
        if len(d["examples"]) < 3:
            d["examples"].append(e["asset"])

    tile_rows = []
    if a.tiles and os.path.exists(a.tiles):
        for line in open(a.tiles, errors="replace"):
            p = line.rstrip("\n").split("\t")
            if len(p) == 4:
                tile_rows.append(p)
    tile_counts = collections.Counter(p[2] for p in tile_rows)
    tile_maps = collections.defaultdict(collections.Counter)
    for m, t, c, d in tile_rows:
        tile_maps[c][m] += 1

    os.makedirs(a.out, exist_ok=True)
    with open(os.path.join(a.out, "mesh-issue-registry.csv"), "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["id", "kind", "asset", "copies", "detector_codes", "evidence", "status"])
        for (key, digest), e in sorted(entries.items(), key=lambda kv: (CLASSES[kv[0][0]][0], kv[1]["asset"])):
            meta = CLASSES[key]
            w.writerow([meta[0], meta[1], e["asset"].lstrip("/"), e["copies"], key, e["metrics"][:160], meta[4]])
        for m, t, c, d in tile_rows:
            if c in TILE_CLASSES and c not in ("SPLINE_RADIUS",):
                meta = TILE_CLASSES[c]
                w.writerow([meta[0], meta[1], f"maps/{m}/{t}", 1, c, d[:160], meta[4]])

    # live coverage
    live_maps, live_veh = [], []
    if a.live and os.path.isdir(a.live):
        for fn in sorted(os.listdir(a.live)):
            if fn.endswith(".json"):
                try:
                    live_maps.append(json.load(open(os.path.join(a.live, fn))))
                except Exception:
                    pass
        vd = os.path.join(a.live, "veh")
        if os.path.isdir(vd):
            for fn in sorted(os.listdir(vd)):
                if fn.endswith(".json"):
                    try:
                        live_veh.append(json.load(open(os.path.join(vd, fn))))
                    except Exception:
                        pass

    out = []
    w = out.append
    w("# Mesh Issue Registry\n")
    w("| Metadata | Value |\n| :--- | :--- |")
    w("| **Generated by** | `scripts/mesh_issues.py` from `mesh_audit --decode`, `tile_audit` and the live sweeps of [P-1014](../../dev/tasks/P-1014 - Scrambled Mesh Audit and Fix.md) |")
    w(f"| **Meshes audited offline** | {total} `.o3d` and `.x` files ({', '.join(f'{k} {v}' for k, v in by_top.most_common(6))}) |")
    w(f"| **Keyed (exporter-scrambled)** | {keyed}; decoded by `decode_confident` {decoded} ({100*decoded/max(keyed,1):.1f} %), rejected {rejected} |")
    w(f"| **Flagged files** | {flagged_files} ({len(entries)} distinct (class, content) entries) |")
    w("| **Full list** | [mesh-issue-registry.csv](mesh-issue-registry.csv) (identical files merged, `copies` = number of installed copies) |\n")
    w("Every irregularity the offline detectors report is catalogued here. `Status`: *fixed* = OPTS bug repaired with a regression test; *tolerance* = deterministic, documented handling of broken content; *content defect* = the source asset is broken and OPTS draws what it can; *benign* / *informational* = the detector fires on legitimate content; *open* = a real defect without a fix.\n")
    w("## Mesh classes (offline audit)\n")
    w("| Id | Kind | Detector | Files (distinct / installed) | Examples | Root cause | Status |\n| :--- | :--- | :--- | ---: | :--- | :--- | :--- |")
    for cid, d in sorted(per_class.items()):
        kind, det, root, status = d["meta"][1], d["meta"][2], d["meta"][3], d["meta"][4]
        ex = "<br>".join(f"`{x.lstrip('/')}`" for x in d["examples"])
        w(f"| {cid} | {kind} | {det} | {d['unique']} / {d['copies']} | {ex} | {root} | {status} |")
    w("")
    if tile_rows or a.tiles:
        w("## Maps, tiles, splines and terrain (offline audit)\n")
        w("| Id | Kind | Findings | Maps most affected | Root cause | Status |\n| :--- | :--- | ---: | :--- | :--- | :--- |")
        seen = set()
        for code, (cid, kind, det, root, status) in TILE_CLASSES.items():
            n = tile_counts.get(code, 0)
            top = ", ".join(f"{m} ({c})" for m, c in tile_maps[code].most_common(3))
            if n == 0 and code not in ("TILE_PARSE_ERROR", "GLOBAL_PARSE_ERROR"):
                continue
            w(f"| {cid} | {kind} | {n} | {top} | {root} | {status} |")
        w("")
    w("## Fixed OPTS bugs found by the audit\n")
    w("| Id | Asset | Symptom | Root cause | Fix and test |\n| :--- | :--- | :--- | :--- | :--- |")
    w("| MI-0002 | `Sceneryobjects/PAD-Labs/Buildings/model/HBF_Koeln.o3d` | file rejected (`unknown chunk 0x7C`) | version 1 stores vertex and face counts as `u16`; the exporter wrote 69,927 vertices as 4,391 | `parse::wrapped_v1_counts` probes `declared + k*65536` against the chunk tags; test `test_parse_v1_vertex_count_wrapped_at_65536` |")
    w("| MI-0003 | `Sceneryobjects/TH_Wald_Objekte/Tristan98/model/Kreisverkehr_02.o3d`, `Sceneryobjects/Ahlheim4_Objekte/model/opnvf5.o3d` | rejected | vertex stream longer than declared / text-transfer damage that also hit a length byte | unresolved content defects (registered, mesh skipped) |")
    w("| MI-0018 | decoded meshes | long spikes through the model | wrongly labelled vertices of accepted decodes | `descramble::drop_spike_faces` (40x median edge, > 1 m); tests `spike_tests`; decode cache version 6 |")
    w("| MI-0020 | maps `Ahlheim 5`, `HB_2017_Bremen-Nord_FIKTIV` (global.cfg) | map cannot be loaded (`invalid number for entrypoint id_code`) | entry point names starting with `#` were dropped as comments, shifting every later record | `global.rs` reads the `[entrypoints]` block from raw lines; test `test_entrypoint_names_may_start_with_hash_or_be_empty` |")
    w("| MI-0021 | maps `Neuendorf` (50 tiles), `Rheinhausen` (10), `Ahlheim 5` (1) | tiles missing in the game | version 10 tiles: one connection field (`-1` = placed, `0` = continues the previous spline, pose fields zero), rules without a flag line | `tile.rs` v10 layout, `spline_end_pose`; tests `test_version_10_splines_are_placed_or_continue_the_previous`, `test_spline_connection_minus_one_means_unconnected` |")
    w("| MI-0027 | wizard (`Plan a drive`) | lists empty when opened during the first scan | the step was built once from the empty list | `rebuild_body` rebuilds when the catalogue size changes; test `a_wizard_opened_on_an_empty_catalogue_fills_when_the_lists_arrive` |")
    w("| MI-0028 | Home after the wizard | `bevy_ecs` panic *Entity despawned* in a deferred command | `rebuild_when_content_arrives` ran only in `MainMenu`; its change detection counted the reload as new on the return and tore down the freshly built page | system runs in every state (see control-protocol architecture page) |")
    w("| MI-0029 | `Vehicles/ADCrealis-Fam/*`, map `DBC_Map` (Chicago), `O405` and 380 materials in all | whole vehicle bodies and a map's buildings were not drawn | `[alphascale] Envir_Brightness` on an *opaque* base material was read as \"hidden by its variable\"; the variable only fades overlays that use alpha | `TextureOverride::is_variable_hidden` (alpha mode or transmap required); test `only_alpha_overlays_are_hidden_by_their_variable`; sco.md `[alphascale]` row |")
    w("| MI-0030 | objects of `Sceneryobjects/DBC_Objects` (`houses_mid_*`) | logged as *object has meshes but none could be loaded* (error) | the same alpha-scale rule dropped every slot | resolved by MI-0029; objects whose slots are all overlay-hidden count as *no geometry*, not as a failure (`hidden_parts`) |")
    w("")
    w("## Field report items (project lead, 2026-10-13; [field report](field-report-2026-10-13.md), [M-0066](../roadmap/phase-10/M-0066.md))\n")
    w("Offline evidence from the audit; none of these four was reproduced to a single mesh yet.\n")
    w("| Id | Report | Audit evidence | Status |\n| :--- | :--- | :--- | :--- |")
    w("| MI-0037 | R-01 parked cars scrambled, missing or transparent | 2,263 car/static-vehicle meshes under `Sceneryobjects`/`Vehicles` match the name pattern; 1,250 are plain and clean, 906 keyed meshes decoded at agreement >= 0.98, **94 decoded at agreement < 0.98 (about half of those are wrong, MI-0018)**, 13 rejected (invisible: MI-0017). Transparent cars: the `[alphascale]` rule (MI-0029) removed bodies of opaque materials until this branch | partly explained (MI-0029 fixed; 94 + 13 meshes open) |")
    w("| MI-0038 | R-09 BRT Citaro UV/mesh | all 1,195 meshes of `BRT_C2-Solo`, `X10_BRT_KI`, `EMT_Citaro`, `KVB_Citaro` are plain (none keyed), no parse, index or extent finding; only texture-density flags (MI-0013, noisy) and 7 missing textures (e.g. `t_c2_frontlampe.tga`, MI-0016). The cause is therefore not the scrambling decoder; candidates are the material/texture binding of those parts | open, needs a visual per-part check |")
    w("| MI-0039 | R-17 buildings flipped / misaligned textures | 8,794 building meshes audited: 7,439 plain, 1,149 decoded >= 0.98, 190 decoded < 0.98, 16 rejected; only 12 files of the corpus are classified `AxisCycled` (axis-swapped layout heuristic, `vertex_layout`). The heuristic and the `Unreadable`/twin path are the places where an axis can be wrong; visual cases `Kbergbahn` (MI-0031) and `DBC_Map` (MI-0034) | open |")
    w("| MI-0040 | R-18 lantern posts scrambled | 2,335 lamp/lantern/neon-light meshes: 1,839 plain, 345 decoded >= 0.98, 100 decoded < 0.98, 51 rejected. The families `Streetobjects_RUE` (`neonlight_*`, `Gaslight_Pole`) and `Lampadaires` are keyed; `light_neon_*_lod` and `gaslight_pole` are rejected (agreement 0.66-0.88), the `light_neon_*` bodies decode at 0.90-1.00 and lose faces to the spike filter (`dropped` 8-16) | cause identified (MI-0017/MI-0018); the 151 meshes are open |")
    w("")
    w("## Live visual observations (no single mesh pinned down)\n")
    w("Seen in the real application on the four-view sweeps (free camera at the start position, 14 m up); the cause is the decoded-mesh accuracy of MI-0018/MI-0019 unless stated.\n")
    w("| Id | Map / vehicle | Observation | Likely cause | Status |\n| :--- | :--- | :--- | :--- | :--- |")
    w("| MI-0031 | map `Kbergbahn` | DDR housing blocks with floating balcony slabs and facade fragments | keyed meshes `ddr_house_medium_06/09`, `ddr_house_small_04/05` decoded without a twin at agreement 0.94-1.00 | open (wrong vertices of accepted decodes) |")
    w("| MI-0032 | map `Grundorf` | grey zig-zag panels next to the start bus stop | houses/signals decoded at agreement 0.93-0.95 (not traced to one mesh) | open |")
    w("| MI-0033 | map `Rheinhausen` | tall grey corrugated wall and two floating grey rectangles at the skyline near the start | not traced (decoded or plain scenery) | open, needs tracing |")
    w("| MI-0034 | map `DBC_Map` | roofs of some blocks carry stretched, concentric texture patterns | `houses_mid_*`/`houses_small_*` decoded with unresolved texture jitter (MI-0019) | open |")
    w("| MI-0035 | map `Lucerne_Line_24_` | a stray white triangle on the grass beside the ship | not traced | open, needs tracing |")
    w("| MI-0036 | maps `Neuendorf`, `Tram`, `Wuppertal_639`, vehicles `MAN_NUE263`, `[SP] Studio Polygon Lite` | loading screen waits for *Continue* (a load with problems); `scene.load_map` / `vehicle.set` return only after it, a script must click `loading.continue` | by design of the load report; `scene.load_map` of a failed load (Ahlheim 5 before the fix) times out after 120 s instead of answering `1008` | open (control protocol follow-up) |")
    w("")
    if a.deteval and os.path.exists(a.deteval):
        w("## How well do the detectors find wrongly decoded meshes?\n")
        w("Measured on plain/keyed twin pairs (ground truth): a decoded mesh is *wrong* when fewer than 98 % of its vertices match the twin.\n")
        w("```")
        w(open(a.deteval).read().strip())
        w("```\n")
        w("Reading: *agreement* is the two annealing runs' share of identical vertex labels; below 0.98 about half of the accepted meshes are wrong, above it about one in ten. The `long-edge rule` lines count the faces longer than k times the median edge that are **still present after** `drop_spike_faces` ran (40x is the rule that is applied; before it existed the same measurement gave 10,373 wrong / 20,169 correct faces beyond 6x, 7,812 / 9,037 beyond 10x, 3,414 / 2,343 beyond 25x and 2,303 / 1,049 beyond 40x, which is why 40x was chosen). The table of detector codes (computed on the filtered decodes, codes that the plain twin itself triggers excluded) shows that the generic geometric detectors find only about one in ten wrongly decoded meshes (recall 0.11) and that texture-coordinate chaos is useless as a signal (precision 0.07): without a plain twin a wrongly decoded mesh is mostly **not** detectable by geometry alone, the agreement is the best available indicator.\n")
    if live_maps or live_veh:
        w("## Live sweeps (real application)\n")
        ok = [m for m in live_maps if "scene" in m]
        failed = [m for m in live_maps if "scene" not in m]
        w(f"Maps loaded in the game and photographed from four free-camera positions: {len(ok)} ok, {len(failed)} failed to load or crashed.\n")
        if ok:
            w("| Map | objects loaded | failed | protected (skipped) | splines failed | load s |\n| :--- | ---: | ---: | ---: | ---: | ---: |")
            for m in ok:
                st = m["scene"].get("stats", {})
                w(f"| {m['map']} | {st.get('objects_loaded', '')} | {st.get('objects_failed', '')} | {st.get('objects_protected', '')} | {st.get('splines_failed', '')} | {m.get('load_s', '')} |")
            w("")
        if failed:
            w("Maps not loaded: " + ", ".join(f"`{m.get('map')}`" for m in failed) + "\n")
        if live_veh:
            vok = [v for v in live_veh if "warns" in v]
            w(f"Vehicles loaded and photographed from four positions: {len(vok)} ok, {len(live_veh) - len(vok)} failed to load.\n")
    open(os.path.join(a.out, "mesh-issue-registry.md"), "w").write("\n".join(out) + "\n")
    print("registry written:", len(entries), "entries,", flagged_files, "flagged files")


if __name__ == "__main__":
    sys.exit(main())
