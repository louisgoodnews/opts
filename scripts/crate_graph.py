#!/usr/bin/env python3
"""Keeps `docs/src/architecture/crate-graph.md` in step with the workspace (M-0074).

The page has a crate table (`Dependencies (Internal)` and `Depended By` columns) and a Mermaid
diagram. Both are compared with `cargo metadata` (normal dependencies between workspace crates):

  * every workspace crate has a table row, and, except the root binary `opts`, a diagram node;
  * the two dependency columns of each row name exactly the internal dependencies and users;
  * the diagram has an edge for every internal dependency and none that does not exist.

`--check` prints the differences and exits 1; `--write` rewrites the two table columns and the
edge block of the diagram (a crate's row and node are written by hand: layer, purpose and
coupling are judgements). Usage: python3 scripts/crate_graph.py [--check | --write]
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PAGE = ROOT / "docs/src/architecture/crate-graph.md"
ROOT_BINARY = "opts"


def workspace() -> dict[str, set[str]]:
    """Crate name to the workspace crates it depends on (normal dependencies, not itself)."""
    meta = json.loads(subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1"],
                                     cwd=ROOT, capture_output=True, text=True, check=True).stdout)
    names = {p["name"] for p in meta["packages"]}
    return {p["name"]: {d["name"] for d in p["dependencies"]
                        if d["name"] in names and d["name"] != p["name"] and d.get("kind") in (None, "normal")}
            for p in meta["packages"]}


def node(name: str) -> str:
    return name.replace("-", "_")


def cell_names(cell: str) -> set[str]:
    return set(re.findall(r"`(opts[\w-]*)`", cell))


def render(names: list[str], old: str) -> str:
    """A dependency cell: the sorted names, `*None*` when empty, and the old parenthetical note."""
    note = re.search(r"\s(\([^)]*\))\s*$", old)
    body = ", ".join(f"`{n}`" for n in sorted(names)) if names else "*None*"
    return body + (f" {note.group(1)}" if note else "")


def main() -> int:
    write = "--write" in sys.argv
    deps = workspace()
    users = {c: {d for d, ds in deps.items() if c in ds} for c in deps}
    text = PAGE.read_text(encoding="utf-8")
    lines = text.splitlines()
    problems: list[str] = []

    rows: dict[str, int] = {}
    for i, l in enumerate(lines):
        m = re.match(r"\|\s*\*\*`([\w-]+)`\*\*\s*\|", l)
        if m:
            rows[m.group(1)] = i
    for c in sorted(deps):
        if c not in rows:
            problems.append(f"no table row for crate {c}")
            continue
        parts = lines[rows[c]].strip().strip("|").split(" | ")
        if len(parts) != 6:
            problems.append(f"row of {c} has {len(parts)} columns, not 6")
            continue
        for col, want, label in ((3, deps[c], "dependencies"), (4, users[c], "depended by")):
            have = cell_names(parts[col])
            if have != want:
                problems.append(f"{c}: {label} are {sorted(have)} in the table, {sorted(want)} in the workspace")
                if write:
                    parts[col] = " " + render(sorted(want), parts[col]) + " "
        if write:
            lines[rows[c]] = "| " + " | ".join(p.strip() for p in parts) + " |"
    for c in rows:
        if c not in deps:
            problems.append(f"table row for {c}, which is not a workspace crate")

    block = re.search(r"```mermaid\n(.*?)```", text, re.S)
    if not block:
        problems.append("no mermaid diagram")
    else:
        diagram = block.group(1)
        for c in sorted(deps):
            if c != ROOT_BINARY and not re.search(rf"\b{node(c)}\[", diagram):
                problems.append(f"no diagram node for crate {c}")
        have = set(re.findall(r"^\s*(opts_\w+)\s+(?:-->|-\.->(?:\|[^|]*\|)?)\s+(opts_\w+)\s*$", diagram, re.M))
        want = {(node(c), node(d)) for c, ds in deps.items() if c != ROOT_BINARY for d in ds}
        for a, b in sorted(want - have):
            problems.append(f"diagram lacks the edge {a} -> {b}")
        for a, b in sorted(have - want):
            problems.append(f"diagram has the edge {a} -> {b}, which the workspace does not")
        if write and (want != have):
            edges = []
            for a, b in sorted(want):
                edges.append(f"    {a} -.->|diagnostics| {b}" if b == "opts_logging" else f"    {a} --> {b}")
            head, _, _ = diagram.partition("    %% Dependencies")
            new = head + "    %% Dependencies\n" + "\n".join(edges) + "\n"
            lines = "\n".join(lines).replace(diagram, new).splitlines()

    if write:
        PAGE.write_text("\n".join(lines) + "\n", encoding="utf-8")
    for p in problems:
        print(("fixed: " if write else "") + p)
    if problems and not write:
        print("crate graph: out of step with the workspace (python3 scripts/crate_graph.py --write)", file=sys.stderr)
        return 1
    print(f"crate graph: {len(deps)} crates, {len(problems)} differences" + (" written" if write and problems else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
