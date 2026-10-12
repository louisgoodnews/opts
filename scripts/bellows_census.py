#!/usr/bin/env python3
"""Census of the articulation bellows of every vehicle in an OMSI installation.

For each `.bus` with `[couple_back]` (and each vehicle whose model uses `articulation_*`
variables) it reads the model `.cfg` and classifies the bellows technique:

* skin    - a `[smoothskin]` mesh with `[setbone]` entries (the bellows is one skinned mesh);
* dummies - meshes carrying `[newanim]` blocks driven by `articulation_*` (the bones, or rigid
            bellows panels);
* both    - the usual combination: skinned bellows bound to animated dummies.

It reports how the dummies are named, which `articulation_*` variables drive them (alpha = yaw,
beta = pitch, gamma = roll), how many bones every skinned mesh lists, whether each bone finds
its dummy by OPTS's rules, and the length of the `[couple_back]` chains.

Usage: scripts/bellows_census.py "<OMSI>/Vehicles" [--csv out.csv]
"""
import collections
import csv
import os
import re
import sys


def rd(p):
    return open(p, "rb").read().decode("latin1").replace("\r", "")


def find(base, rel):
    cur = base
    for c in re.split(r"[\\/]", rel.strip()):
        if not c:
            continue
        try:
            names = os.listdir(cur)
        except OSError:
            return None
        hit = [n for n in names if n.lower() == c.lower()]
        if not hit:
            return None
        cur = os.path.join(cur, hit[0])
    return cur


def blocks(text):
    """(keyword, [params]) in file order; parameters are the following non-empty lines."""
    out = []
    lines = text.split("\n")
    i = 0
    while i < len(lines):
        l = lines[i].strip()
        m = re.fullmatch(r"\[([A-Za-z_0-9]+)\]", l)
        if m:
            params = []
            j = i + 1
            while j < len(lines) and not lines[j].strip().startswith("[") and len(params) < 40:
                if lines[j].strip():
                    params.append(lines[j].strip())
                j += 1
            out.append((m.group(1).lower(), params))
        i += 1
    return out


HUMAN_BONES = {"hip", "main", "head", "os_l", "os_r", "us_l", "us_r", "oa_l", "oa_r", "ua_l", "ua_r", "hand_l", "hand_r"}


def bone_stem(stem):
    return bool(re.fullmatch(r"(bone_|dummy_|dummy)[a-z]", stem))


def analyse_model(cfg_path):
    text = rd(cfg_path)
    meshes = []  # dicts
    cur = None
    for kw, params in blocks(text):
        if kw == "mesh" and params:
            cur = {"path": params[0], "bones": [], "skin": False, "anim_vars": set()}
            meshes.append(cur)
        elif cur is not None:
            if kw == "smoothskin":
                cur["skin"] = True
            elif kw == "setbone" and len(params) >= 2:
                cur["bones"].append(params[0].lower())
            elif kw == "newanim":
                for k, a in enumerate(params):
                    if a.lower() in ("anim_rot", "anim_trans", "anim_rot_x", "anim_rot_y", "anim_rot_z") and k + 1 < len(params):
                        v = params[k + 1].lower()
                        if v.startswith("articulation_"):
                            cur["anim_vars"].add(v)
    return meshes


def main():
    root = sys.argv[1]
    csv_path = sys.argv[sys.argv.index("--csv") + 1] if "--csv" in sys.argv else None
    buses = []
    for d, _, fs in os.walk(root):
        for f in fs:
            if f.lower().endswith(".bus"):
                buses.append(os.path.join(d, f))
    parsed = {}
    for b in buses:
        t = rd(b)
        m = re.search(r"^\[model\]\s*\n([^\n]+)", t, re.M | re.I)
        cb = re.search(r"^\[couple_back\]\s*\n([^\n]+)", t, re.M | re.I)
        parsed[b] = (m.group(1).strip() if m else None, cb.group(1).strip() if cb else None)
    technique = collections.Counter()
    variables = collections.Counter()
    varset_vehicles = collections.defaultdict(set)
    dummy_names = collections.Counter()
    unbound = []
    rows = []
    families = collections.defaultdict(lambda: collections.Counter())
    chain_len = collections.Counter()

    def chain(b, seen=()):
        cb = parsed[b][1]
        if not cb or b in seen:
            return 1
        nb = find(os.path.dirname(b), cb)
        if not nb or nb not in parsed:
            return 1
        return 1 + chain(nb, seen + (b,))

    for b, (model, cb) in parsed.items():
        if not model:
            continue
        cfg = find(os.path.dirname(b), model)
        if not cfg or not os.path.isfile(cfg):
            continue
        meshes = analyse_model(cfg)
        skins = [m for m in meshes if m["skin"] and m["bones"]]
        anim = [m for m in meshes if m["anim_vars"]]
        if not (skins or anim or cb):
            continue
        tech = "both" if skins and anim else "skin" if skins else "dummies" if anim else "none"
        fam = os.path.relpath(b, root).split(os.sep)[0]
        technique[(("articulated" if cb else "single"), tech)] += 1
        families[fam][tech] += 1
        if cb:
            chain_len[chain(b)] += 1
        for m in anim:
            for v in m["anim_vars"]:
                variables[v] += 1
                varset_vehicles[v].add(fam)
            stem = os.path.splitext(re.split(r"[\\/]", m["path"])[-1])[0].lower()
            dummy_names[re.sub(r"[a-d0-9]$", "#", stem)] += 1
        # Binding by OPTS's rules: a bone binds when an animated mesh stem equals/contains the
        # bone name, or an animated bone-like dummy run exists before the skinned mesh.
        stems = [os.path.splitext(re.split(r"[\\/]", m["path"])[-1])[0].lower() for m in meshes]
        keys = [
            (st[:-1] if m["anim_vars"] and len(st) >= 3 and st[-1].isalnum() else None)
            for st, m in zip(stems, meshes)
        ]
        in_run = [False] * len(meshes)
        for i in range(1, len(meshes)):
            if keys[i] is not None and keys[i] == keys[i - 1]:
                in_run[i] = in_run[i - 1] = True
        dummies = [st for st, m in zip(stems, meshes) if m["anim_vars"]]
        has_run = any(in_run) or any(bone_stem(st) for st in dummies)
        for s in skins:
            for bone in s["bones"]:
                if bone in HUMAN_BONES:
                    continue
                ok = any(bone in d for d in dummies) or has_run
                if not ok:
                    unbound.append((fam, os.path.basename(b), bone))
        rows.append((fam, os.path.basename(b), "articulated" if cb else "single", tech, len(skins), sum(len(s["bones"]) for s in skins), ",".join(sorted({v for m in anim for v in m["anim_vars"]}))))
    print("== technique per vehicle (model .cfg of each .bus)")
    for k, v in sorted(technique.items()):
        print(f"  {k[0]:11} {k[1]:8} {v}")
    print("== articulation chain length (units per [couple_back] chain, counted per head file)")
    for k, v in sorted(chain_len.items()):
        print(f"  {k} units: {v}")
    print("== driving variables (vehicle files / distinct vehicle folders)")
    for v, n in variables.most_common(24):
        print(f"  {n:4} {len(varset_vehicles[v]):3}  {v}")
    print("== animated dummy file names (last letter/digit folded)")
    for n, c in dummy_names.most_common(16):
        print(f"  {c:4} {n}")
    print(f"== bones without an animated dummy (OPTS rule): {len(unbound)}")
    for fam, bus, bone in unbound[:25]:
        print(f"  {fam} / {bus}: {bone}")
    print("== vehicle folders by technique")
    for fam, c in sorted(families.items()):
        print(f"  {fam:40} {dict(c)}")
    if csv_path:
        with open(csv_path, "w", newline="") as fh:
            w = csv.writer(fh)
            w.writerow(["folder", "bus", "kind", "technique", "skinned_meshes", "bones", "variables"])
            w.writerows(sorted(rows))


if __name__ == "__main__":
    main()
