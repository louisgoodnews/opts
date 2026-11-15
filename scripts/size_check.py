#!/usr/bin/env python3
"""Reports files over 150 lines of code and functions over 80 lines of code (standards.md §3.7, §1.6).

Usage: scripts/size_check.py [--fail]   (--fail: exit 1 when there are violations)

A read-only view of the same measurements `scripts/structure_audit.py` uses (it imports them, so
the two cannot disagree): code lines exclude comments and blank lines, test modules count, and
generated and table files are skipped. Entries grandfathered in docs/dev/structure-known.txt and
docs/dev/function-known.txt are listed too; `structure_audit.py check` is the gate that decides
what fails.
"""
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import structure_audit as sa  # noqa: E402

files, fns = [], []
for info in sa.collect():
    if info.marker == "table":
        continue
    if info.code > sa.FILE_LIMIT:
        files.append((info.code, info.path))
    for fn in info.fns:
        if fn.code > sa.FN_LIMIT:
            fns.append((fn.code, f"{info.path}:{fn.start} {fn.name}"))
files.sort(reverse=True)
fns.sort(reverse=True)
print(f"{len(files)} file(s) over {sa.FILE_LIMIT} lines of code, {len(fns)} function(s) over {sa.FN_LIMIT} lines of code")
for n, f in files[:20]:
    print(f"  file {n:5d}  {f}")
for n, f in fns[:60]:
    print(f"  fn   {n:5d}  {f}")
if "--fail" in sys.argv and (files or fns):
    sys.exit(1)
