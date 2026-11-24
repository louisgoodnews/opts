#!/usr/bin/env python3
"""Census of the event sources in an installed OMSI data tree (M-0118, Phase A).

Reads the scripts (`Vehicles/`, `Addons/`: `.osc`, `model*.cfg`, `sound*.cfg`) and the key files
(`Inputs/keyboard.cfg`, `gamectrler.cfg`, `keyboard_reset.cfg`) under the data root given as the
argument, and prints a count table: sites and distinct names per source kind, and the
classification counts of events.md section 3.2. Only counts are printed, never an event name or
script text. Files are read as bytes (latin-1), so no encoding can stop the scan. Nothing is
written unless `--out FILE` is given, and the output path must lie outside the repository.

Opt-in identifier list: `--names-out FILE` writes the sorted, distinct identifiers of every name
category (raised, payload, sound, key, mouse and trigger-block names), one per line, and only
names that match `[a-z0-9_]+`; the count of names left out by that rule goes to stderr. The
default run and `--out` never print or write names. Owner decision (2026-11-24): this list may be
committed as `docs/src/formats/text/vehicles/event-identifiers.txt`.

Usage: python3 scripts/omsi_event_census.py <data-root> [--out FILE] [--names-out FILE]
"""
import argparse
import re
import sys
from pathlib import Path

EVENT = re.compile(r"\(\s*T\.([LFS])\.([^)]*)\)", re.IGNORECASE)
TRIGGER_BLOCK = re.compile(r"\{\s*trigger\s*:([^}]*)\}", re.IGNORECASE)
LITERAL = re.compile(r"^[a-z0-9_]+$")
KEY_FILES = ("keyboard.cfg", "gamectrler.cfg", "keyboard_reset.cfg")


def first_after(lines: list[str], header: str) -> list[str]:
    """The first non-blank line after each `[header]` line, lowercased (case-insensitive)."""
    out = []
    for i, line in enumerate(lines):
        if line.strip().lower() == f"[{header}]":
            nxt = next((x for x in lines[i + 1:] if x.strip()), None)
            if nxt is not None:
                out.append(nxt.strip().lower())
    return out


def scan_script(text: str) -> tuple[dict[str, list[str]], int, list[str]]:
    """Literal event sites per kind (L, F, S), the count of non-literal accessors, and the
    trigger-block names of one `.osc` text."""
    sites: dict[str, list[str]] = {"L": [], "F": [], "S": []}
    dynamic = 0
    for kind, name in EVENT.findall(text):
        name = name.strip().lower()
        if LITERAL.match(name):
            sites[kind.upper()].append(name)
        else:
            dynamic += 1
    blocks = [b.strip().lower() for b in TRIGGER_BLOCK.findall(text)]
    return sites, dynamic, blocks


def census(root: Path) -> dict:
    """Walk the data root and collect every source list; the names stay internal."""
    c: dict = {"files": 0, "L": [], "F": [], "S": [], "dynamic": 0, "blocks": [],
               "sound": [], "keys": [], "mouse": []}
    for sub in ("Vehicles", "Addons"):
        base = root / sub
        if not base.is_dir():
            continue
        for p in sorted(base.rglob("*")):
            name = p.name.lower()
            if not p.is_file() or not name.endswith((".osc", ".cfg")):
                continue
            text = read_text(p)
            if name.endswith(".osc"):
                sites, dyn, blocks = scan_script(text)
                for kind in ("L", "F", "S"):
                    c[kind].extend(sites[kind])
                c["dynamic"] += dyn
                c["blocks"].extend(blocks)
            elif name.startswith("sound"):
                c["sound"].extend(first_after(text.splitlines(), "trigger"))
            elif name.startswith("model"):
                c["mouse"].extend(first_after(text.splitlines(), "mouseevent"))
            else:
                continue
            c["files"] += 1
    for name in KEY_FILES:
        p = root / "Inputs" / name
        if p.is_file():
            c["files"] += 1
            c["keys"].extend(first_after(read_text(p).splitlines(), "entry"))
    return c


def read_text(p: Path) -> str:
    return p.read_bytes().decode("latin-1")


def classify(c: dict) -> list[tuple[str, int]]:
    """Counts behind events.md section 3.2 (names are compared, never printed)."""
    raised, sound = set(c["L"]), set(c["sound"])
    keys, mouse = set(c["keys"]), set(c["mouse"])
    derived = {k + "_off" for k in keys} | {m + s for m in mouse for s in ("_off", "_drag")}
    known = raised | sound | keys | mouse | derived
    residual = [b for b in c["blocks"] if b not in known]
    return [
        ("raised names (T.L)", len(raised)),
        ("  raised and bound by a sound trigger", len(raised & sound)),
        ("  raised, no sound binding", len(raised - sound)),
        ("payload names (T.F or T.S)", len(set(c["F"]) | set(c["S"]))),
        ("sound names never raised as T.L", len(sound - raised)),
        ("trigger-block names", len(set(c["blocks"]))),
        ("residual names (engine-raised candidates)", len(set(residual))),
        ("residual block sites", len(residual)),
    ]


def identifiers(c: dict) -> tuple[list[str], int]:
    """Sorted distinct identifiers of every name category, and the count of names left out
    because they do not match the identifier pattern."""
    names = set(c["L"]) | set(c["F"]) | set(c["S"]) | set(c["sound"]) | set(c["keys"])
    names |= set(c["mouse"]) | set(c["blocks"])
    kept = sorted(n for n in names if LITERAL.match(n))
    return kept, len(names) - len(kept)


def render(c: dict, rows: list[tuple[str, int]]) -> str:
    sources = [
        ("files scanned", c["files"]),
        ("(T.L) sites", len(c["L"])),
        ("(T.F) sites", len(c["F"])),
        ("(T.S) sites", len(c["S"])),
        ("non-literal event accessors", c["dynamic"]),
        ("sound trigger entries", len(c["sound"])),
        ("key entries (distinct)", len(set(c["keys"]))),
        ("mouse trigger entries (distinct)", len(set(c["mouse"]))),
    ]
    lines = ["| Count | Value |", "| :--- | ---: |"]
    lines += [f"| {k} | {v:,} |" for k, v in sources]
    lines += ["", "| Classification (events.md 3.2) | Count |", "| :--- | ---: |"]
    lines += [f"| {k} | {v:,} |" for k, v in rows]
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("data_root", type=Path)
    ap.add_argument("--out", type=Path)
    ap.add_argument("--names-out", type=Path,
                    help="opt-in: write the sorted identifier list (one per line) to this file")
    args = ap.parse_args(argv)
    if not args.data_root.is_dir():
        print("data root not found", file=sys.stderr)
        return 2
    c = census(args.data_root)
    text = render(c, classify(c))
    if args.out:
        args.out.write_text(text, encoding="utf-8")
    if args.names_out:
        kept, left_out = identifiers(c)
        args.names_out.write_text("".join(n + "\n" for n in kept), encoding="utf-8")
        print(f"identifiers written: {len(kept)} (left out by the pattern: {left_out})",
              file=sys.stderr)
    print(text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
