#!/usr/bin/env python3
"""Regenerate the book's copies of the canonical ROADMAP.md.

`ROADMAP.md` (repository root) is canonical. Generated from it:

* `docs/src/roadmap/README.md` -- the whole file, with links rewritten relative to
  `docs/src/roadmap/` and the pointer line replaced by the book's pointer;
* the checklist of every `docs/src/roadmap/phase-N/README.md` -- everything after the
  line containing "Mirrored from" is replaced by the body of ROADMAP's "Phase N" section
  (up to its closing `---`), links rewritten relative to the phase folder.

Usage: `python3 scripts/gen_roadmap_copy.py` writes the files; `--check` only compares and
exits with status 1 if any copy is out of date (used in CI).
"""

import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROADMAP = "ROADMAP.md"
BOOK_COPY = "docs/src/roadmap/README.md"
BOOK_POINTER = (
    "> 📌 This page is the book's copy of the repository-root [`ROADMAP.md`](../../../ROADMAP.md), "
    "which is canonical. Edit `ROADMAP.md` and regenerate this page "
    "(`python3 scripts/gen_roadmap_copy.py`; links are rewritten relative to `docs/src/roadmap/`)."
)
LINK = re.compile(r"(\]\s*)\(([^)\s]+)\)")


def rewrite_links(text: str, target_dir: str) -> str:
    """Rewrites repository-relative link targets so they resolve from `target_dir`."""

    def fix(m: re.Match) -> str:
        target = m.group(2)
        if re.match(r"^(https?:|mailto:|#)", target):
            return m.group(0)
        path, _, frag = target.partition("#")
        new = os.path.relpath(os.path.normpath(path), target_dir)
        return f"{m.group(1)}({new}{'#' + frag if frag else ''})"

    return LINK.sub(fix, text)


def book_copy(roadmap: str) -> str:
    lines = roadmap.split("\n")
    pointer = next(i for i, l in enumerate(lines) if l.startswith("> 📌"))
    body = rewrite_links("\n".join(lines[:pointer] + ["\0"] + lines[pointer + 1 :]), "docs/src/roadmap")
    return body.replace("\0", BOOK_POINTER)


def phase_sections(roadmap: str) -> dict:
    """Maps phase number -> body lines of its ROADMAP section (heading and closing `---` excluded)."""
    sections, current, buf = {}, None, []
    for line in roadmap.split("\n"):
        m = re.match(r"^## .*Phase (\d+):", line)
        if m:
            current, buf = int(m.group(1)), []
            continue
        if current is not None:
            if line.strip() == "---" or line.startswith("## "):
                sections[current] = buf
                current = None
            else:
                buf.append(line)
    if current is not None:
        sections[current] = buf
    return sections


def phase_readme(old: str, body: list, phase_dir: str) -> str:
    lines = old.split("\n")
    idx = next((i for i, l in enumerate(lines) if "Mirrored from" in l), None)
    if idx is None:
        return old
    text = "\n".join(body).strip("\n")
    return "\n".join(lines[: idx + 1]) + "\n\n" + rewrite_links(text, phase_dir) + "\n"


def main() -> int:
    check = "--check" in sys.argv[1:]
    os.chdir(ROOT)
    roadmap = open(ROADMAP, encoding="utf-8").read()
    outputs = {BOOK_COPY: book_copy(roadmap)}
    for n, body in phase_sections(roadmap).items():
        path = f"docs/src/roadmap/phase-{n}/README.md"
        if os.path.exists(path):
            outputs[path] = phase_readme(open(path, encoding="utf-8").read(), body, os.path.dirname(path))
    stale = []
    for path, new in outputs.items():
        old = open(path, encoding="utf-8").read() if os.path.exists(path) else ""
        if old != new:
            stale.append(path)
            if not check:
                with open(path, "w", encoding="utf-8") as f:
                    f.write(new)
    if check and stale:
        print("out of date (run python3 scripts/gen_roadmap_copy.py):", *stale, sep="\n  ")
        return 1
    print(("up to date" if check else f"wrote {len(stale)} file(s)") if not stale or not check else "")
    return 0


if __name__ == "__main__":
    sys.exit(main())
