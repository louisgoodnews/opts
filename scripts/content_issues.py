#!/usr/bin/env python3
"""Builds the content issue registry from OPTS logs.

Reads the per-subsystem log files (`opts-bevy`, `opts-game`, `opts-sim`, `opts-core`) of the OPTS
log directory, keeps WARN and ERROR events since a cutoff, groups them by *subject* (the splines
`.sli`, scenery objects `.sco`/`.o3d`, vehicle scripts `.osc`, vehicles, textures, fonts, ...) and
*message class* (numbers removed) and writes

  docs/src/quality/content-issue-registry.md   summary tables (top entries per category)
  docs/src/quality/content-issue-registry.csv  every entry

Usage: scripts/content_issues.py --since 2026-10-12T09:16:00Z [--logs DIR] [--out DIR]

The registry is meant to be regenerated after each sweep (run the game on every map, then this
script) and diffed: an entry that disappears is resolved, a new one is a regression. Entries
carry the maps and vehicles of the sessions they appeared in when the log shows them.
"""
import argparse
import collections
import csv
import glob
import os
import re
import sys

LINE = re.compile(r"^(\d{4}-\d\d-\d\dT[\d:.]+Z)\s+(ERROR|WARN)\s+(\S+)\s+(.*)$")
EXT = "sli|sco|o3d|osc|bus|cfg|ovh|hum|x|bmp|dds|tga|jpg|png|oft|txt|map|terrain|wav|ogg"
# Paths inside the OMSI installation (the folder names contain spaces).
OMSI_ROOT = re.compile(r"/[^\s{},=]*/OMSI 2/", re.I)
PATH = re.compile(r"\x00([^{},=\x00]*?\.(?:" + EXT + r"))(?=[\s,}:]|$)", re.I)
FIELD = re.compile(r"\{([a-z_]+)=([^}]*)\}")

CATEGORIES = [
    ("splines", ("sli",)),
    ("scenery objects", ("sco",)),
    ("meshes", ("o3d", "x")),
    ("vehicle scripts", ("osc",)),
    ("vehicles", ("bus", "ovh", "cfg", "hum")),
    ("textures", ("bmp", "dds", "tga", "jpg", "png")),
    ("fonts", ("oft",)),
]


def classify(subject: str, target: str, msg: str) -> str:
    ext = subject.rsplit(".", 1)[-1].lower() if "." in subject else ""
    if "vehicle_script" in target:
        return "vehicle scripts"
    if "spline" in msg.lower() and not ext:
        return "splines"
    for name, exts in CATEGORIES:
        if ext in exts:
            return name
    if "pedestrian" in target:
        return "pedestrians"
    if "traffic" in target:
        return "traffic"
    return "other"


def normalise(msg: str) -> str:
    msg = re.sub(r"\{log\..*$", "", msg)
    msg = FIELD.sub("", msg)
    msg = PATH.sub("<file>", OMSI_ROOT.sub("\x00", msg))
    msg = re.sub(r"\(op \d+\)", "(op N)", msg)
    msg = re.sub(r"\d+(\.\d+)?", "N", msg)
    return re.sub(r"\s+", " ", msg).strip()[:140]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--since", required=True, help="ISO time (UTC); older events are ignored")
    ap.add_argument("--logs", default=os.path.expanduser("~/.local/share/opts/logs"))
    ap.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "..", "docs", "src", "quality"))
    ap.add_argument("--top", type=int, default=40)
    args = ap.parse_args()

    files = []
    for sub in ("bevy", "game", "sim", "core"):
        files += sorted(glob.glob(os.path.join(args.logs, f"opts-{sub}.log.*")))
    entries = {}
    total = 0
    for path in files:
        with open(path, errors="replace") as fh:
            for line in fh:
                m = LINE.match(line)
                if not m or m.group(1) < args.since:
                    continue
                ts, sev, target, rest = m.groups()
                total += 1
                paths = PATH.findall(OMSI_ROOT.sub("\x00", rest))
                fields = dict(FIELD.findall(rest))
                subject = paths[0] if paths else ""
                if not subject:
                    for key in ("bone", "folder", "character", "texture", "font"):
                        if key in fields:
                            subject = f"{key}={fields[key]}"
                            break
                if "vehicle_script" in target and not subject:
                    subject = "(script engine)"
                cls = normalise(rest)
                # Script problems are keyed by file and macro, not by op index.
                key = (classify(subject, target, rest), subject, sev, cls)
                e = entries.setdefault(key, {"count": 0, "first": ts, "last": ts, "ctx": set()})
                e["count"] += 1
                e["last"] = max(e["last"], ts)
                for k in ("vehicle", "map", "tile"):
                    if k in fields:
                        e["ctx"].add(f"{k}={fields[k]}")

    os.makedirs(args.out, exist_ok=True)
    rows = sorted(entries.items(), key=lambda kv: (-kv[1]["count"], kv[0]))
    with open(os.path.join(args.out, "content-issue-registry.csv"), "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["category", "subject", "severity", "message", "count", "first_seen", "last_seen", "context"])
        for (cat, subj, sev, cls), e in rows:
            w.writerow([cat, subj, sev, cls, e["count"], e["first"], e["last"], ";".join(sorted(e["ctx"]))[:200]])

    by_cat = collections.defaultdict(list)
    for (cat, subj, sev, cls), e in rows:
        by_cat[cat].append((subj, sev, cls, e))
    with open(os.path.join(args.out, "content-issue-registry.md"), "w") as fh:
        fh.write("# Content Issue Registry\n\n")
        fh.write("| Metadata | Value |\n| :--- | :--- |\n")
        fh.write(f"| **Generated by** | `scripts/content_issues.py --since {args.since}` |\n")
        fh.write(f"| **Events read** | {total} WARN/ERROR events, {len(entries)} distinct entries |\n")
        fh.write("| **Full list** | [content-issue-registry.csv](content-issue-registry.csv) |\n\n")
        fh.write("Splines, scenery objects, meshes, vehicle scripts, vehicles and textures that log a warning or an error "
                 "in the current build. Regenerate after a sweep of all maps; an entry that vanishes is resolved, a new "
                 "one is a regression. The `Status` column is kept by hand in the CSV workflow of the next issue pass "
                 "(`open` here).\n\n")
        for cat in [c for c, _ in CATEGORIES] + ["pedestrians", "traffic", "other"]:
            items = by_cat.get(cat)
            if not items:
                continue
            fh.write(f"## {cat.capitalize()} ({len(items)} entries, {sum(i[3]['count'] for i in items)} events)\n\n")
            fh.write("| Subject | Sev | Message | Count | Context | Status |\n| :--- | :--- | :--- | ---: | :--- | :--- |\n")
            for subj, sev, cls, e in items[: args.top]:
                ctx = ", ".join(sorted(e["ctx"]))[:80]
                fh.write(f"| `{subj}` | {sev} | {cls.replace('|', '/')} | {e['count']} | {ctx} | open |\n")
            if len(items) > args.top:
                fh.write(f"\n… {len(items) - args.top} more in the CSV.\n")
            fh.write("\n")
    print(f"{total} events, {len(entries)} entries -> {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
