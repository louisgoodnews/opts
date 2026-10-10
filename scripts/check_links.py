#!/usr/bin/env python3
"""Fails when a relative markdown link in the repository points at a missing file.

Checks every `*.md` under `docs/` plus the root `*.md` files. External (`http`, `mailto`) links,
pure fragments, code blocks, and the paths listed in `IGNORED` (templates with placeholder links)
are skipped. Run from anywhere: `python3 scripts/check_links.py`.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# Files whose links are deliberate placeholders.
IGNORED = {
    "docs/src/adr/template.md",
    "docs/dev/Documentation Freeze — Task Prompt.md",
}


def main() -> int:
    os.chdir(ROOT)
    files = glob.glob("docs/**/*.md", recursive=True) + glob.glob("*.md")
    broken = []
    for f in sorted(files):
        if f in IGNORED:
            continue
        text = open(f, encoding="utf-8", errors="replace").read()
        text = re.sub(r"```.*?```", "", text, flags=re.S)
        for m in re.finditer(r"\]\(([^)\s]+)\)", text):
            url = m.group(1)
            if url.startswith(("http://", "https://", "mailto:", "#")):
                continue
            path = url.split("#", 1)[0]
            if not path:
                continue
            if not os.path.exists(os.path.normpath(os.path.join(os.path.dirname(f), path))):
                broken.append((f, url))
    for f, url in broken:
        print(f"{f}: broken link {url}")
    print(f"{len(broken)} broken link(s) in {len(files)} files")
    return 1 if broken else 0


if __name__ == "__main__":
    sys.exit(main())
