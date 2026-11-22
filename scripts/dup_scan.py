#!/usr/bin/env python3
"""Duplication scan and unused-pub check over the workspace crates (ISS-035, measured, not enforced).

  dup_scan.py dup [--top N]    windows of 8 code lines (comments, blanks, `use` and attributes
                               removed) that occur at two or more places; runs of consecutive
                               matching windows are merged and ranked by length.
  dup_scan.py unused-pub       `pub` fn/struct/enum/trait/type/const/static items whose name
                               appears nowhere else in the workspace (a lower bound: a common
                               name counts as used).
Run: python3 scripts/dup_scan.py dup; tests: python3 scripts/test_dup_scan.py"""
import argparse
import collections
import hashlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WINDOW = 8  # code lines per window
MIN_CHARS = 60  # windows with less text (braces, short calls) are not counted
MAX_BUCKET = 12  # a window seen more often than this is boilerplate, not a cluster
PUB = re.compile(r"^\s*pub\s+(?:async\s+|const\s+|unsafe\s+)*(fn|struct|enum|trait|type|const|static)\s+(\w+)")
IDENT = re.compile(r"\b\w+\b")


def rust_files(root: Path) -> list[Path]:
    return sorted(p for p in (root / "crates").rglob("*.rs") if "target" not in p.parts)


def code_lines(path: Path) -> list[tuple[int, str]]:
    """(line number, normalised text) of the code lines; `//` comments are cut naively."""
    out, in_block = [], False
    for n, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        line = raw.strip()
        if in_block:
            if "*/" not in line:
                continue
            in_block, line = False, line.split("*/", 1)[1].strip()
        if line.startswith("/*") and "*/" not in line:
            in_block = True
            continue
        line = re.sub(r"//.*", "", line).strip()
        if not line or line.startswith(("use ", "#[", "#![")):
            continue
        out.append((n, re.sub(r"\s+", " ", line)))
    return out


def windows(root: Path) -> dict[str, list[tuple[str, int, int]]]:
    """Map a window hash to the (file, first code-line index, first physical line) where it occurs."""
    index = collections.defaultdict(list)
    for path in rust_files(root):
        lines = code_lines(path)
        rel = path.relative_to(root).as_posix()
        for i in range(len(lines) - WINDOW + 1):
            text = "\n".join(t for _, t in lines[i : i + WINDOW])
            if len(text) >= MIN_CHARS:
                index[hashlib.sha1(text.encode()).hexdigest()].append((rel, i, lines[i][0]))
    return index


def clusters(root: Path) -> list[dict]:
    """Merged runs of duplicated windows: one dict per pair of files and code-line offset, longest first.
    Offsets count code lines, so a comment inserted in one copy does not split the run."""
    runs = collections.defaultdict(list)  # (file a, file b, code offset) -> (index a, line a, line b)
    for places in windows(root).values():
        if not 2 <= len(places) <= MAX_BUCKET:
            continue
        for i, p in enumerate(places):
            for q in places[i + 1 :]:
                (fa, ia, na), (fb, ib, nb) = sorted([p, q])
                if fa == fb and ib - ia < WINDOW:
                    continue
                runs[(fa, fb, ib - ia)].append((ia, na, nb))
    out = []
    for (fa, fb, offset), hits in runs.items():
        hits = sorted(set(hits))
        first = prev = hits[0]
        count = 0
        for h in hits[1:] + [None]:
            if h is not None and h[0] == prev[0] + 1:
                prev, count = h, count + 1
                continue
            out.append(dict(a=fa, a_line=first[1], b=fb, b_line=first[2], lines=count + WINDOW))
            if h is not None:
                first = prev = h
                count = 0
    return sorted(out, key=lambda c: -c["lines"])


def cmd_dup(root: Path, top: int) -> int:
    found = clusters(root)
    print(f"{len(found)} duplicated runs; top {min(top, len(found))} by length (code lines):")
    for c in found[:top]:
        print(f"{c['lines']:4d}  {c['a']}:{c['a_line']}  =  {c['b']}:{c['b_line']}")
    return 0


def cmd_unused(root: Path) -> int:
    defs = []
    counts = collections.Counter()
    for path in rust_files(root):
        rel = path.relative_to(root).as_posix()
        for n, raw in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
            counts.update(IDENT.findall(raw))
            m = PUB.match(raw)
            if m:
                defs.append((m.group(2), m.group(1), rel, n))
    defined = collections.Counter(name for name, *_ in defs)
    unused = [(kind, name, rel, n) for name, kind, rel, n in defs if counts[name] - defined[name] == 0]
    print(f"{len(unused)} of {len(defs)} pub items have no reference outside their definition line:")
    for kind, name, rel, n in unused:
        print(f"{kind:7s} {name:32s} {rel}:{n}")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["dup", "unused-pub"])
    ap.add_argument("--top", type=int, default=15)
    ap.add_argument("--root", type=Path, default=ROOT, help="measure another tree (tests)")
    a = ap.parse_args(argv)
    if a.command == "dup":
        return cmd_dup(a.root.resolve(), a.top)
    return cmd_unused(a.root.resolve())


if __name__ == "__main__":
    sys.exit(main())
