#!/usr/bin/env python3
"""Census of wheel, steering and suspension animations (`[newanim]`) of an OMSI installation.

Usage: scripts/wheel_census.py "<OMSI>/Vehicles" [--csv out.csv]

For every model `.cfg` of every `.bus` it lists the `[newanim]` blocks that drive
`Wheel_Rotation_*`, `Axle_Steering_*`, `Axle_Suspension_*` (and steering-wheel variables) with
their origin rotation, so the axis conventions can be compared across the fleet.
"""
import collections
import csv
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from bellows_census import find, rd  # noqa: E402

KINDS = [
    ("wheel", re.compile(r"^wheel_rotation_(\d+)_([lr])$")),
    ("steer", re.compile(r"^axle_steering_(\d+)_([lr])$")),
    ("susp", re.compile(r"^axle_suspension_(\d+)_([lr])$")),
]
WHEEL_WORDS = re.compile(r"lenkrad|steer|lenk")


def anims(text):
    """Yield (mesh_path, [params-of-newanim as list of lines]) in order."""
    mesh = None
    cur = None
    out = []
    for raw in text.split("\n"):
        l = raw.strip()
        low = l.lower()
        if low == "[mesh]":
            mesh = "?"
            cur = None
            want_mesh = True
            out.append(("mesh", None))
            continue
        if out and out[-1] == ("mesh", None):
            out[-1] = ("mesh", l)
            continue
        if low == "[newanim]":
            cur = []
            out.append(("anim", cur))
            continue
        if low.startswith("[") and low.endswith("]") and cur is not None:
            cur = None
        if cur is not None and l:
            cur.append(l)
    return out


def parse_anim(lines):
    d = {"rot": [0.0, 0.0, 0.0], "trans": None, "from_mesh": False, "acts": []}
    i = 0
    while i < len(lines):
        k = lines[i].lower()
        try:
            if k == "origin_from_mesh":
                d["from_mesh"] = True
            elif k in ("origin_rot_x", "origin_rot_y", "origin_rot_z"):
                d["rot"]["xyz".index(k[-1])] = float(lines[i + 1].replace(",", "."))
                i += 1
            elif k == "origin_trans":
                d["trans"] = tuple(float(x.replace(",", ".")) for x in lines[i + 1 : i + 4])
                i += 3
            elif k in ("anim_rot", "anim_trans"):
                d["acts"].append((k, lines[i + 1].lower(), float(lines[i + 2].replace(",", "."))))
                i += 2
        except (ValueError, IndexError):
            pass
        i += 1
    return d


def main():
    root = sys.argv[1]
    csv_path = sys.argv[sys.argv.index("--csv") + 1] if "--csv" in sys.argv else None
    seen_cfg = set()
    rows = []
    triples = collections.defaultdict(collections.Counter)  # kind -> (rot tuple, act) counts
    per_family = collections.defaultdict(lambda: collections.defaultdict(set))
    for d, _, fs in os.walk(root):
        for f in fs:
            if not f.lower().endswith(".bus"):
                continue
            b = os.path.join(d, f)
            t = rd(b)
            m = re.search(r"^\[model\]\s*\n([^\n]+)", t, re.M | re.I)
            if not m:
                continue
            cfg = find(d, m.group(1))
            if not cfg or not os.path.isfile(cfg) or cfg in seen_cfg:
                continue
            seen_cfg.add(cfg)
            fam = os.path.relpath(b, root).split(os.sep)[0]
            mesh = ""
            for kind, payload in anims(rd(cfg)):
                if kind == "mesh":
                    mesh = payload or ""
                    continue
                a = parse_anim(payload)
                for act, var, factor in a["acts"]:
                    for name, rx in KINDS:
                        mm = rx.match(var)
                        if mm:
                            key = (name, tuple(a["rot"]), act, "mesh" if a["from_mesh"] else "trans" if a["trans"] else "none")
                            triples[name][key] += 1
                            per_family[name][key].add(fam)
                            rows.append((fam, os.path.basename(cfg), mesh, name, var, a["rot"], act, factor, a["from_mesh"], a["trans"]))
                            break
                    else:
                        if WHEEL_WORDS.search(var) and not var.startswith("axle_"):
                            key = ("cockpit-steer", tuple(a["rot"]), act, var)
                            triples["cockpit-steer"][key] += 1
                            per_family["cockpit-steer"][key].add(fam)
    for name in ("wheel", "steer", "susp", "cockpit-steer"):
        print(f"== {name}: frame rotation (x,y,z) | action | origin | count | folders")
        for key, n in triples[name].most_common(14):
            print(f"  {str(key[1]):22} {key[2]:10} {str(key[3])[:28]:28} {n:6}  {len(per_family[name][key]):3}")
    if csv_path:
        with open(csv_path, "w", newline="") as fh:
            w = csv.writer(fh)
            w.writerow(["folder", "cfg", "mesh", "kind", "variable", "rot", "action", "factor", "from_mesh", "trans"])
            w.writerows(rows)


if __name__ == "__main__":
    main()
