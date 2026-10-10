#!/usr/bin/env python3
"""Reports files over 500 lines and functions over 80 lines in crates/*/src (standards.md).

Usage: scripts/size_check.py [--fail]   (--fail: exit 1 when there are violations)
Function length is measured from the `fn` line to its closing brace (brace counting; strings
and comments containing braces can skew it by a line or two). Test modules are skipped.
"""
import pathlib
import re
import sys

FILE_MAX, FN_MAX = 500, 80
root = pathlib.Path(__file__).resolve().parent.parent
files, fns = [], []
for path in sorted(root.glob("crates/*/src/**/*.rs")):
    lines = path.read_text(errors="replace").splitlines()
    rel = path.relative_to(root)
    if len(lines) > FILE_MAX:
        files.append((len(lines), str(rel)))
    in_tests = None
    i = 0
    while i < len(lines):
        line = lines[i]
        if re.match(r"\s*#\[cfg\(test\)\]", line):
            in_tests = len(line) - len(line.lstrip())
        m = re.match(r"(\s*)(pub(\([a-z]+\))?\s+)?(const\s+)?(async\s+)?(unsafe\s+)?fn\s+(\w+)", line)
        if m and in_tests is None:
            depth, started, j = 0, False, i
            while j < len(lines):
                code = re.sub(r'"(\\.|[^"\\])*"', '""', lines[j].split("//")[0])
                depth += code.count("{") - code.count("}")
                started = started or "{" in code
                if started and depth <= 0:
                    break
                if not started and code.rstrip().endswith(";"):
                    break
                j += 1
            length = j - i + 1
            if started and length > FN_MAX:
                fns.append((length, f"{rel}:{i + 1} {m.group(7)}"))
        i += 1
files.sort(reverse=True)
fns.sort(reverse=True)
print(f"{len(files)} file(s) over {FILE_MAX} lines, {len(fns)} function(s) over {FN_MAX} lines")
for n, f in files:
    print(f"  file {n:5d}  {f}")
for n, f in fns[:60]:
    print(f"  fn   {n:5d}  {f}")
if "--fail" in sys.argv and (files or fns):
    sys.exit(1)
