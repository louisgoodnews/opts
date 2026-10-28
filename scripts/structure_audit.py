#!/usr/bin/env python3
"""Structure audit of the Rust code (M-0125, M-0147): file/function sizes, inline tests, allows, crates.

A repeatable measurement of the codebase's structure, in the units of `docs/src/contributing/
standards.md` (§3.7 and §3.14), so a refactoring cycle (Phase 15 / Phase 999) can show that it
improved something and pre-commit / CI can refuse a regression.

Sub-commands (Python 3, no dependencies):

  report [--date YYYY-MM-DD] [--out FILE]  write the Markdown report (default
                                           docs/src/meta/structure-audit-<date>.md)
  check                                    the ratchet: exit 1 when a file, inline-test file,
                                           function (> 80 LOC), or signature allow grew beyond what
                                           docs/dev/structure-known.txt or
                                           docs/dev/function-known.txt records
  json                                     the measurements as JSON (for a cycle comparison)
  seed                                     (re)write docs/dev/structure-known.txt and
                                           docs/dev/function-known.txt from the tree, keeping the
                                           reasons of entries that already exist
  show PATH...                             total, code, comment, blank and test lines of files
                                           (the before and after counts of a commit message)

MEASUREMENT RULES (standards.md §3.7 & §3.14):

  * a *blank* line has only whitespace; a *comment* line has no code outside comments (line
    comments `//`, doc comments `///` and `//!`, block comments `/* */`, nested); every other line
    is *code*, including a line with code and a trailing comment, and a line inside a string.
  * the in-file unit tests (`#[cfg(test)] mod tests { .. }`) **count**: the standard says "lines of
    code" of the file; any production file (`crates/*/src/**/*.rs`, excluding `tests.rs`) over
    250 code lines that contains inline test code (`test_code > 0`) must move its `#[cfg(test)]`
    module to a sibling `tests.rs` file (Rule R2).
  * the hard file limit is 350 lines of code per file (Rule R1; target 150-200) and 80 lines of
    code per function (Rule R3; target 40), plus signature-complexity allows
    (`too_many_arguments`, `type_complexity`, Rule R4).
  * *total* lines (what `wc -l` counts) are reported next to *code*, so the figures of the phase
    README (which are totals) can be compared: the difference is comments and blanks.

EXCLUDED as generated or fixture: a file whose first 5 lines contain `structure-audit: generated`
or `@generated`, any file below a `golden/`, `fixtures/` or `target/` directory, and `*.generated.rs`.
A long lookup table that must stay one file carries the marker `structure-audit: table` in its
first 5 lines: it is measured and listed, but not held against the limit.
"""
from __future__ import annotations

import argparse
import datetime
import json
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KNOWN = ROOT / "docs/dev/structure-known.txt"
FN_KNOWN = ROOT / "docs/dev/function-known.txt"
FILE_LIMIT = 350
INLINE_TEST_LIMIT = 250
FN_LIMIT = 80
FN_TARGET = 40
REPORT_FILE_FLOOR = 350
FN_FLOOR = 60
SKIP_DIRS = {"golden", "fixtures", "target", ".git"}
PUB_ITEM = re.compile(r"^\s*pub\s+(?:async\s+|const\s+|unsafe\s+|extern\s+\"C\"\s+)*(fn|struct|enum|trait|type|const|static|mod)\b")
FN_NAME = re.compile(r"(?<![A-Za-z0-9_])fn\s+([A-Za-z_][A-Za-z0-9_]*)")
SIG_ALLOW = re.compile(r"#!?\[allow\([^]]*\b(?:clippy::)?(?:too_many_arguments|type_complexity)\b")
CODE_STOP = re.compile(r'[/\'"]')
BRACE = re.compile(r"[{}]")


# --------------------------------------------------------------------------------------------
# Lexer: classify lines and blank out comments and string contents.
# --------------------------------------------------------------------------------------------
def lex(text: str) -> tuple[list[str], list[str]]:
    """Per line: its class ('code', 'comment', 'blank') and its text with comments and the
    contents of strings and character literals replaced by spaces (quotes kept)."""
    out: list[str] = []
    n = len(text)
    i = 0
    state = "code"
    depth = 0  # block comment nesting
    raw_hashes = 0
    while i < n:
        if state == "code":
            m = CODE_STOP.search(text, i)
            if m is None:
                out.append(text[i:])
                break
            if m.start() > i:
                out.append(text[i : m.start()])
                i = m.start()
            c = text[i]
            two = text[i : i + 2]
            if two == "//":
                nl = text.find("\n", i + 2)
                if nl == -1:
                    out.append(" " * (n - i))
                    break
                out.append(" " * (nl - i) + "\n")
                i = nl + 1
            elif two == "/*":
                state = "block"
                depth = 1
                out.append("  ")
                i += 2
            elif c == '"':
                j = i - 1
                hashes = 0
                while j >= 0 and text[j] == "#":
                    hashes += 1
                    j -= 1
                if j >= 0 and text[j] == "r" and (j == 0 or not (text[j - 1].isalnum() or text[j - 1] == "_") or text[j - 1] == "b"):
                    state = "rawstr"
                    raw_hashes = hashes
                else:
                    state = "str"
                out.append('"')
                i += 1
            elif c == "'":
                if text[i + 1 : i + 2] == "\\":
                    j = text.find("'", i + 2)
                    j = j if j != -1 else i + 1
                    out.append("'" + " " * (j - i - 1) + "'")
                    i = j + 1
                elif i + 2 < n and text[i + 2] == "'" and text[i + 1] != "'":
                    out.append("' '")
                    i += 3
                else:
                    out.append("'")
                    i += 1
            else:
                out.append(c)
                i += 1
        elif state == "block":
            two = text[i : i + 2]
            if two == "/*":
                depth += 1
                out.append("  ")
                i += 2
            elif two == "*/":
                depth -= 1
                out.append("  ")
                i += 2
                if depth == 0:
                    state = "code"
            else:
                out.append("\n" if text[i] == "\n" else " ")
                i += 1
        elif state == "str":
            c = text[i]
            if c == "\\" and i + 1 < n:
                out.append("  " if text[i + 1] != "\n" else " \n")
                i += 2
            elif c == '"':
                state = "code"
                out.append('"')
                i += 1
            else:
                out.append("\n" if c == "\n" else " ")
                i += 1
        else:  # rawstr
            c = text[i]
            if c == '"' and text[i + 1 : i + 1 + raw_hashes] == "#" * raw_hashes:
                state = "code"
                out.append('"' + " " * raw_hashes)
                i += 1 + raw_hashes
            else:
                out.append("\n" if c == "\n" else " ")
                i += 1
    cleaned = "".join(out).split("\n")
    orig = text.split("\n")
    classes = [
        "blank" if (k < len(orig) and not orig[k].strip()) else ("code" if line.strip() else "comment")
        for k, line in enumerate(cleaned)
    ]
    return classes, cleaned


# --------------------------------------------------------------------------------------------
# Measurements
# --------------------------------------------------------------------------------------------
@dataclass
class Fn:
    name: str
    start: int
    end: int
    code: int


@dataclass
class FileInfo:
    path: str
    crate: str
    total: int = 0
    code: int = 0
    comment: int = 0
    blank: int = 0
    test_code: int = 0
    marker: str = ""
    fns: list[Fn] = field(default_factory=list)
    has_module_doc: bool = True
    undocumented_pub: list[int] = field(default_factory=list)
    allow_without_reason: list[int] = field(default_factory=list)
    signature_allows: int = 0
    unwraps: list[int] = field(default_factory=list)
    tests: int = 0
    unbalanced: bool = False
    is_test_file: bool = False
    is_library_path: bool = True


def balanced_end(cleaned_text: str, open_at: int) -> int:
    """Index of the `}` matching the `{` at `open_at`, or -1."""
    depth = 0
    for m in BRACE.finditer(cleaned_text, open_at):
        if m.group() == "{":
            depth += 1
        else:
            depth -= 1
            if depth == 0:
                return m.start()
    return -1


def functions(cleaned_lines: list[str], classes: list[str]) -> tuple[list[Fn], bool]:
    text = "\n".join(cleaned_lines)
    line_start = [0]
    for ln in cleaned_lines[:-1]:
        line_start.append(line_start[-1] + len(ln) + 1)

    def line_of(pos: int) -> int:
        lo, hi = 0, len(line_start) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if line_start[mid] <= pos:
                lo = mid
            else:
                hi = mid - 1
        return lo

    found: list[Fn] = []
    unbalanced = False
    for m in FN_NAME.finditer(text):
        k = m.end()
        paren = 0
        body = -1
        while k < len(text):
            ch = text[k]
            if ch in "([":
                paren += 1
            elif ch in ")]":
                paren -= 1
            elif ch == ";" and paren <= 0:
                break
            elif ch == "{" and paren <= 0:
                body = k
                break
            k += 1
        if body < 0:
            continue
        end = balanced_end(text, body)
        if end < 0:
            unbalanced = True
            continue
        s, e = line_of(m.start()), line_of(end)
        code = sum(1 for x in range(s, e + 1) if classes[x] == "code")
        found.append(Fn(m.group(1), s + 1, e + 1, code))
    return found, unbalanced


def cfg_test_lines(cleaned_lines: list[str]) -> set[int]:
    """0-based line numbers inside `#[cfg(test)] mod name { .. }` blocks."""
    text = "\n".join(cleaned_lines)
    line_start = [0]
    for ln in cleaned_lines[:-1]:
        line_start.append(line_start[-1] + len(ln) + 1)
    inside: set[int] = set()
    for m in re.finditer(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", text):
        open_at = m.end() - 1
        end = balanced_end(text, open_at)
        if end < 0:
            continue

        def line_of(pos: int) -> int:
            lo, hi = 0, len(line_start) - 1
            while lo < hi:
                mid = (lo + hi + 1) // 2
                if line_start[mid] <= pos:
                    lo = mid
                else:
                    hi = mid - 1
            return lo

        inside.update(range(line_of(m.start()), line_of(end) + 1))
    return inside


def marker_of(text: str) -> str:
    head = "\n".join(text.split("\n")[:5])
    if "structure-audit: generated" in head or "@generated" in head:
        return "generated"
    if "structure-audit: table" in head:
        return "table"
    return ""


def measure(path: Path) -> FileInfo | None:
    rel = path.relative_to(ROOT).as_posix()
    parts = rel.split("/")
    if any(p in SKIP_DIRS for p in parts) or rel.endswith(".generated.rs"):
        return None
    text = path.read_text(encoding="utf-8", errors="replace")
    mark = marker_of(text)
    if mark == "generated":
        return None
    crate = parts[1] if len(parts) > 1 and parts[0] == "crates" else parts[0]
    info = FileInfo(path=rel, crate=crate, marker=mark)
    classes, cleaned = lex(text)
    orig = text.split("\n")
    if orig and orig[-1] == "":
        classes, cleaned, orig = classes[:-1], cleaned[:-1], orig[:-1]
    info.total = len(orig)
    info.code = classes.count("code")
    info.comment = classes.count("comment")
    info.blank = classes.count("blank")
    first = next((ln for ln in orig if ln.strip()), "")
    info.has_module_doc = first.lstrip().startswith("//!")
    info.fns, info.unbalanced = functions(cleaned, classes)
    in_test = cfg_test_lines(cleaned)
    info.test_code = sum(1 for k in in_test if k < len(classes) and classes[k] == "code")
    name = parts[-1]
    in_src = len(parts) > 3 and parts[2] == "src"
    info.is_test_file = ("tests" in parts and not in_src) or name == "tests.rs" or "tests" in parts[3:-1]
    info.is_library_path = in_src and not info.is_test_file and name != "main.rs" and "bin" not in parts[3:-1]
    info.tests = len(re.findall(r"#\[(?:tokio::)?test\b", text))
    for k, line in enumerate(cleaned):
        if k in in_test or not in_src:
            continue
        if PUB_ITEM.match(line):
            j = k - 1
            while j >= 0 and orig[j].strip().startswith("#["):
                j -= 1
            prev = orig[j].strip() if j >= 0 else ""
            if not (prev.startswith("///") or prev.endswith("*/") or prev.startswith("#[doc")):
                info.undocumented_pub.append(k + 1)
    for k, line in enumerate(orig):
        if re.search(r"#!?\[allow\(", line) and k not in in_test:
            prev = orig[k - 1].strip() if k else ""
            if "//" not in line.split("]", 1)[-1] and not prev.startswith("//"):
                info.allow_without_reason.append(k + 1)
        if SIG_ALLOW.search(cleaned[k]) and k not in in_test:
            info.signature_allows += 1
    if info.is_library_path:
        for k, line in enumerate(cleaned):
            if k in in_test:
                continue
            if re.search(r"\.unwrap\(\)|\.expect\(", line):
                info.unwraps.append(k + 1)
    return info


def rust_files() -> list[Path]:
    out = []
    for p in sorted((ROOT / "crates").rglob("*.rs")):
        if any(x in SKIP_DIRS for x in p.relative_to(ROOT).parts):
            continue
        out.append(p)
    return out


def collect() -> list[FileInfo]:
    return [i for i in (measure(p) for p in rust_files()) if i]


def crate_table(files: list[FileInfo]) -> list[dict]:
    rows = []
    by_crate: dict[str, list[FileInfo]] = defaultdict(list)
    for f in files:
        by_crate[f.crate].append(f)
    for crate, fs in sorted(by_crate.items()):
        src = [f for f in fs if f.path.startswith(f"crates/{crate}/src/")]
        code = sum(f.code for f in src)
        pub_items = 0
        for f in src:
            text = (ROOT / f.path).read_text(encoding="utf-8", errors="replace")
            pub_items += len(PUB_ITEM_ALL.findall(text))
        depth = max((len(Path(f.path).relative_to(f"crates/{crate}/src").parts) for f in src), default=0)
        manifest = ROOT / "crates" / crate / "Cargo.toml"
        ext, internal = [], []
        if manifest.exists():
            sect = ""
            for line in manifest.read_text(encoding="utf-8").splitlines():
                line = line.strip()
                if line.startswith("["):
                    sect = line
                elif sect == "[dependencies]" and "=" in line and not line.startswith("#"):
                    d = line.split("=")[0].strip()
                    (internal if d.startswith("opts-") else ext).append(d)
        notes = []
        lib = ROOT / "crates" / crate / "src" / "lib.rs"
        if code and code < 1000:
            notes.append("under 1,000 lines of code")
        if code > 10000:
            notes.append("over 10,000 lines of code")
        if lib.exists() and not lib.read_text(encoding="utf-8", errors="replace").lstrip().startswith("//!"):
            notes.append("lib.rs has no `//!` document")
        tops = [p.stem for p in (ROOT / "crates" / crate / "src").glob("*.rs")] if (ROOT / "crates" / crate / "src").exists() else []
        short = crate.replace("opts-", "").replace("-", "_")
        if short in tops and short not in ("lib", "main"):
            notes.append(f"top-level module `{short}` shadows the crate name")
        rows.append(dict(crate=crate, files=len(src), code=code, pub_items=pub_items, depth=depth,
                         external=sorted(ext), internal=sorted(internal), notes=notes))
    return rows


PUB_ITEM_ALL = re.compile(r"^\s*pub\s+(?:async\s+|const\s+|unsafe\s+)*(?:fn|struct|enum|trait|type|const|static|mod)\b", re.M)


def module_table(files: list[FileInfo]) -> list[dict]:
    dirs: dict[str, list[FileInfo]] = defaultdict(list)
    for f in files:
        d = str(Path(f.path).parent.as_posix())
        if "/src" in d or d.endswith("/src"):
            dirs[d].append(f)
    rows = []
    for d, fs in sorted(dirs.items()):
        big = max(fs, key=lambda x: x.code)
        rows.append(dict(dir=d, files=len(fs), code=sum(f.code for f in fs), largest=big.path.split("/")[-1], largest_code=big.code,
                         single=len(fs) == 1 and Path(d).name != "src"))
    return rows


# --------------------------------------------------------------------------------------------
# Ratchet (R1: FILE_LIMIT=350, R2: INLINE_TEST_LIMIT=250 w/ test_code>0, R3: FN_LIMIT=80, R4: allows)
# --------------------------------------------------------------------------------------------
def has_inline_test_violation(f: FileInfo) -> bool:
    return f.is_library_path and f.code > INLINE_TEST_LIMIT and f.test_code > 0 and f.marker != "table"


def over_limit(files: list[FileInfo]) -> dict[str, int]:
    return {
        f.path: f.code
        for f in files
        if (f.code > FILE_LIMIT or has_inline_test_violation(f)) and f.marker != "table"
    }


def fn_and_allow_over_limit(files: list[FileInfo]) -> dict[str, int]:
    out: dict[str, int] = {}
    for f in files:
        if f.marker == "table":
            continue
        for fn in f.fns:
            if fn.code > FN_LIMIT:
                key = f"{f.path}::{fn.name}"
                out[key] = max(out.get(key, 0), fn.code)
        if f.signature_allows > 0:
            out[f"{f.path}::@allow"] = f.signature_allows
    return out


def read_known_file(path: Path) -> tuple[dict[str, int], dict[str, str]]:
    sizes, reasons = {}, {}
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.startswith("#"):
                continue
            parts = re.split(r"\s+#", line, maxsplit=1)
            body = parts[0]
            reason = parts[1].strip() if len(parts) > 1 else ""
            key, num = body.split()
            sizes[key] = int(num)
            reasons[key] = reason
    return sizes, reasons


def read_known() -> tuple[dict[str, int], dict[str, str]]:
    return read_known_file(KNOWN)


def read_fn_known() -> tuple[dict[str, int], dict[str, str]]:
    return read_known_file(FN_KNOWN)


def cmd_check(files: list[FileInfo]) -> int:
    by_path = {f.path: f for f in files}
    now = over_limit(files)
    known, _ = read_known()
    bad = 0
    for path, code in sorted(now.items()):
        f = by_path[path]
        if path not in known:
            if f.code <= FILE_LIMIT and has_inline_test_violation(f):
                print(
                    f"FAIL {path}: {code} lines of code (with {f.test_code} in #[cfg(test)]), over the "
                    f"{INLINE_TEST_LIMIT}-line inline-test limit and not in docs/dev/structure-known.txt "
                    f"(move #[cfg(test)] to tests.rs)"
                )
            else:
                print(
                    f"FAIL {path}: {code} lines of code, over the {FILE_LIMIT}-line limit and not in "
                    f"docs/dev/structure-known.txt (split it by concern, or record it with a reason)"
                )
            bad += 1
        elif code > known[path]:
            print(
                f"FAIL {path}: grew from {known[path]} to {code} lines of code (it is over the "
                f"{FILE_LIMIT}-line / {INLINE_TEST_LIMIT}-line inline-test limit; known files may only shrink)"
            )
            bad += 1
        elif code < known[path]:
            print(f"ok   {path}: {code} (known {known[path]}: lower the ratchet with `structure_audit.py seed`)")
    for path in sorted(set(known) - set(now)):
        print(f"ok   {path}: no longer over the limit (remove it from docs/dev/structure-known.txt with `seed`)")

    fn_now = fn_and_allow_over_limit(files)
    fn_known, _ = read_fn_known()
    for key, val in sorted(fn_now.items()):
        is_allow = key.endswith("::@allow")
        if key not in fn_known:
            if is_allow:
                path = key[: -len("::@allow")]
                print(
                    f"FAIL {path}: {val} signature-complexity #[allow] attribute(s) "
                    f"(too_many_arguments/type_complexity), not in docs/dev/function-known.txt "
                    f"(bundle parameters into #[derive(SystemParam)])"
                )
            else:
                print(
                    f"FAIL {key}: {val} lines of code, over the {FN_LIMIT}-line function limit and not in "
                    f"docs/dev/function-known.txt (extract helpers <= {FN_TARGET} lines)"
                )
            bad += 1
        elif val > fn_known[key]:
            unit = "signature-complexity #[allow](s)" if is_allow else "lines of code"
            print(f"FAIL {key}: grew from {fn_known[key]} to {val} {unit} (known entries may only shrink)")
            bad += 1
        elif val < fn_known[key]:
            print(f"ok   {key}: {val} (known {fn_known[key]}: lower the ratchet with `structure_audit.py seed`)")
    for key in sorted(set(fn_known) - set(fn_now)):
        print(f"ok   {key}: no longer over the limit (remove it from docs/dev/function-known.txt with `seed`)")

    over_350 = sum(1 for f in files if f.code > FILE_LIMIT and f.marker != "table")
    inline_250 = sum(1 for f in files if has_inline_test_violation(f))
    fns_80 = sum(1 for k in fn_now if not k.endswith("::@allow"))
    allows = sum(v for k, v in fn_now.items() if k.endswith("::@allow"))
    print(
        f"structure: {over_350} files over {FILE_LIMIT} LOC, {inline_250} prod files > {INLINE_TEST_LIMIT} LOC "
        f"with inline tests, {fns_80} functions > {FN_LIMIT} LOC, {allows} signature allows, {bad} regression(s)"
    )
    return 1 if bad else 0


def cmd_seed(files: list[FileInfo]) -> int:
    by_path = {f.path: f for f in files}
    now = over_limit(files)
    _, reasons = read_known()
    lines = [
        f"# Files over the {FILE_LIMIT}-line limit (R1) or production files over {INLINE_TEST_LIMIT} lines with inline",
        "# #[cfg(test)] modules (R2, standards.md §3.7 & §3.14) that are tolerated for now.",
        "# Format: <path> <lines of code>  # <reason or planned milestone>. A known file may only shrink;",
        f"# a new file over {FILE_LIMIT} lines (or > {INLINE_TEST_LIMIT} lines with inline tests) fails `structure_audit.py check`.",
        "# Regenerate (keeping the reasons) with `python3 scripts/structure_audit.py seed`.",
    ]
    for path, code in sorted(now.items()):
        f = by_path[path]
        if path in reasons and reasons[path]:
            reason = reasons[path]
        elif has_inline_test_violation(f) and f.code - f.test_code <= FILE_LIMIT:
            reason = f"planned: M-0148 batch 1 (extract {f.test_code} inline test lines to tests.rs)"
        elif has_inline_test_violation(f):
            reason = f"planned: M-0148 + M-0149 (extract {f.test_code} test lines to tests.rs, then split > {FILE_LIMIT} LOC)"
        elif "/examples/" in path or "/tests/" in path or path.endswith("/tests.rs"):
            reason = "planned: M-0149 batch 2 (oversized test or example file: split by concern)"
        else:
            reason = f"planned: M-0149 batch 2 (split production file > {FILE_LIMIT} LOC along concern boundaries)"
        lines.append(f"{path} {code}  # {reason}")
    KNOWN.write_text("\n".join(lines) + "\n", encoding="utf-8")

    fn_now = fn_and_allow_over_limit(files)
    _, fn_reasons = read_fn_known()
    fn_lines = [
        f"# Functions over the {FN_LIMIT}-line limit (R3) and per-file signature-complexity #[allow] counts",
        "# (too_many_arguments / type_complexity, R4, standards.md §3.7 & §3.14) tolerated for now.",
        "# Format: <path>::<fn_name_or_@allow> <code_lines_or_count>  # <reason>. Entries may only shrink.",
        "# Regenerate (keeping the reasons) with `python3 scripts/structure_audit.py seed`.",
    ]
    for key, val in sorted(fn_now.items()):
        if key in fn_reasons and fn_reasons[key]:
            reason = fn_reasons[key]
        elif key.endswith("::@allow"):
            reason = "planned: M-0150 batch 3 (bundle system parameters into #[derive(SystemParam)])"
        else:
            reason = f"planned: M-0150 batch 3 (decompose function > {FN_LIMIT} LOC into <= {FN_TARGET}-line helpers)"
        fn_lines.append(f"{key} {val}  # {reason}")
    FN_KNOWN.write_text("\n".join(fn_lines) + "\n", encoding="utf-8")
    print(f"wrote {KNOWN.relative_to(ROOT)}: {len(now)} files; {FN_KNOWN.relative_to(ROOT)}: {len(fn_now)} entries")
    return 0


# --------------------------------------------------------------------------------------------
# Report
# --------------------------------------------------------------------------------------------
def md_table(head: list[str], rows: list[list]) -> str:
    out = ["| " + " | ".join(head) + " |", "| " + " | ".join(":---" if i == 0 else "---:" for i in range(len(head))) + " |"]
    out += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return "\n".join(out)


def concern(name: str) -> str:
    return name.strip("_").split("_")[0] or name


def cmd_report(files: list[FileInfo], date: str, out: Path | None) -> int:
    total_lines = sum(f.total for f in files)
    code = sum(f.code for f in files)
    big = sorted(files, key=lambda f: -f.code)
    thresholds = [250, 350, 400, 500, 800, 1000]
    by_total = {t: sum(1 for f in files if f.total > t) for t in thresholds}
    by_code = {t: sum(1 for f in files if f.code > t) for t in thresholds}
    all_fns = [(f, fn) for f in files for fn in f.fns]
    fn_over = sorted(((f, fn) for f, fn in all_fns if fn.code > FN_FLOOR), key=lambda x: -x[1].code)
    parts: list[str] = []
    parts.append(f"# Structure Audit {date}\n")
    parts.append(f"*Generated by `python3 scripts/structure_audit.py report` on {date}. Rules, units and blind spots: the header of the script (`scripts/structure_audit.py`).*\n")
    parts.append("## 1. Summary\n")
    parts.append(md_table(["Measure", "Value"], [
        [".rs files measured", len(files)],
        ["total lines (`wc -l`)", f"{total_lines:,}"],
        ["lines of code (no comments, no blanks)", f"{code:,}"],
        ["comment lines / blank lines", f"{sum(f.comment for f in files):,} / {sum(f.blank for f in files):,}"],
        [f"files over {FILE_LIMIT} lines of code (R1 hard ceiling)", by_code[FILE_LIMIT]],
        [f"production files over {INLINE_TEST_LIMIT} lines of code with inline tests (R2)", sum(1 for f in files if has_inline_test_violation(f))],
        ["functions over 80 lines of code (R3 hard limit)", sum(1 for _, fn in all_fns if fn.code > FN_LIMIT)],
        ["functions over 40 lines of code (R3 target)", sum(1 for _, fn in all_fns if fn.code > FN_TARGET)],
        ["signature-complexity `#[allow]` attributes (R4)", sum(f.signature_allows for f in files)],
    ]))
    parts.append("\nFiles above a size, counted two ways. The Phase 999 README counts **total** lines; the standard counts **lines of code**; the difference is comments and blank lines.\n")
    parts.append(md_table(["Over", "total lines", "lines of code"], [[t, by_total[t], by_code[t]] for t in thresholds]))
    parts.append("\n## 2. The five largest\n")
    parts.append("**Files** (lines of code):\n")
    parts.append(md_table(["File", "code", "total", "of which tests"], [[f"`{f.path}`", f.code, f.total, f.test_code] for f in big[:5]]))
    parts.append("\n**Functions** (lines of code):\n")
    parts.append(md_table(["Function", "code", "lines"], [[f"`{f.path}` `{fn.name}`", fn.code, f"{fn.start}-{fn.end}"] for f, fn in fn_over[:5]]))
    crates = crate_table(files)
    parts.append("\n**Crates** (lines of code in `src`):\n")
    parts.append(md_table(["Crate", "code", "files"], [[f"`{c['crate']}`", f"{c['code']:,}", c["files"]] for c in sorted(crates, key=lambda c: -c["code"])[:5]]))
    mods = module_table(files)
    parts.append("\n**Module directories** (lines of code):\n")
    parts.append(md_table(["Directory", "code", "files", "largest file"], [[f"`{m['dir']}`", f"{m['code']:,}", m["files"], f"`{m['largest']}` ({m['largest_code']})"] for m in sorted(mods, key=lambda m: -m["code"])[:5]]))
    parts.append(f"\n## 3. Every file over {REPORT_FILE_FLOOR} lines of code\n")
    rows = [[f"`{f.path}`", f.code, f.total, f.comment, f.blank, f.test_code, f.marker or ("**over**" if f.code > FILE_LIMIT else "")] for f in big if f.code > REPORT_FILE_FLOOR]
    parts.append(md_table(["File", "code", "total", "comment", "blank", "tests in file", "note"], rows))
    parts.append(f"\n## 4. Every function over {FN_FLOOR} lines of code\n")
    parts.append("The *concern* is the first word of the function's name: a quick way to see what kind of function it is (`build`, `spawn`, `parse`).\n")
    parts.append(md_table(["Function", "code", "lines", "concern", "limit"], [[f"`{f.path}` `{fn.name}`", fn.code, f"{fn.start}-{fn.end}", concern(fn.name), "**over 80**" if fn.code > FN_LIMIT else "over 60"] for f, fn in fn_over]))
    parts.append("\n## 5. Modules\n")
    parts.append(f"{len(mods)} directories with Rust sources; {sum(1 for m in mods if m['single'])} hold a single file (a module that is a directory for one file).\n")
    parts.append(md_table(["Directory", "files", "code", "largest file"], [[f"`{m['dir']}`", m["files"], f"{m['code']:,}", f"`{m['largest']}` ({m['largest_code']})"] for m in sorted(mods, key=lambda m: -m["code"])[:25]]))
    singles = [m["dir"] for m in mods if m["single"]]
    if singles:
        parts.append("\nSingle-file module directories: " + ", ".join(f"`{d}`" for d in singles) + "\n")
    tops = [f for f in files if re.fullmatch(r"crates/[^/]+/src/[^/]+\.rs", f.path) and not f.path.endswith(("lib.rs", "main.rs")) and f.code > REPORT_FILE_FLOOR]
    if tops:
        parts.append(f"\nFiles at a crate's top level over {REPORT_FILE_FLOOR} lines of code that probably belong in a submodule: " + ", ".join(f"`{f.path}` ({f.code})" for f in tops) + "\n")
    parts.append("\n## 6. Crates\n")
    parts.append(md_table(["Crate", "files", "code", "pub items", "module depth", "external dependencies", "notes"], [[f"`{c['crate']}`", c["files"], f"{c['code']:,}", c["pub_items"], c["depth"], ", ".join(c["external"]) or "none", "; ".join(c["notes"]) or ""] for c in crates]))
    parts.append("\n## 7. Standards violations (textual scan)\n")
    lib_files = [f for f in files if f.path.split("/")[2:3] == ["src"]]
    parts.append(md_table(["Rule", "count", "worst files"], [
        [f"file over {FILE_LIMIT} lines of code (3.7 / 3.14)", by_code[FILE_LIMIT], ", ".join(f"`{f.path.split('/')[-1]}` {f.code}" for f in big[:3])],
        [f"function over {FN_LIMIT} lines of code", sum(1 for _, fn in all_fns if fn.code > FN_LIMIT), ", ".join(f"`{fn.name}` {fn.code}" for _, fn in fn_over[:3])],
        ["module file without a `//!` document (3.8)", sum(1 for f in lib_files if not f.has_module_doc), ", ".join(f"`{f.path}`" for f in [f for f in lib_files if not f.has_module_doc][:3])],
        ["`pub` item without a `///` document (3.9)", sum(len(f.undocumented_pub) for f in lib_files), ", ".join(f"`{f.path.split('/')[-1]}` {len(f.undocumented_pub)}" for f in sorted(lib_files, key=lambda f: -len(f.undocumented_pub))[:3] if f.undocumented_pub)],
        ["`#[allow]` without a justification comment", sum(len(f.allow_without_reason) for f in files), ", ".join(f"`{f.path.split('/')[-1]}` {len(f.allow_without_reason)}" for f in sorted(files, key=lambda f: -len(f.allow_without_reason))[:3] if f.allow_without_reason)],
        ["`unwrap()` / `expect(` in library paths", sum(len(f.unwraps) for f in files), ", ".join(f"`{f.path.split('/')[-1]}` {len(f.unwraps)}" for f in sorted(files, key=lambda f: -len(f.unwraps))[:3] if f.unwraps)],
    ]))
    unb = [f.path for f in files if f.unbalanced]
    if unb:
        parts.append("\nFiles whose braces did not balance for the function scan (measure by hand): " + ", ".join(f"`{p}`" for p in unb) + "\n")
    parts.append("\n## 8. Test files over 500 lines\n")
    tests = sorted((f for f in files if f.is_test_file and f.total > 500), key=lambda f: -f.total)
    parts.append(md_table(["File", "total", "code", "`#[test]` count"], [[f"`{f.path}`", f.total, f.code, f.tests] for f in tests]) if tests else "None.")
    parts.append("\n## 9. How to read this\n")
    parts.append(f"The numbers come from a textual scan (blind spots in the script header). Pre-commit and CI run `check`, which enforces the four Phase 15 ratchets (`docs/dev/structure-known.txt` and `docs/dev/function-known.txt`): files over {FILE_LIMIT} lines of code, production files over {INLINE_TEST_LIMIT} lines with inline tests, functions over {FN_LIMIT} lines, and signature-complexity `#[allow]` attributes. Known items may shrink freely.\n")
    target = out or ROOT / f"docs/src/meta/structure-audit-{date}.md"
    target.write_text("\n".join(parts) + "\n", encoding="utf-8")
    print(f"wrote {target.relative_to(ROOT) if target.is_relative_to(ROOT) else target}: {len(files)} files, {code:,} lines of code, {by_code[FILE_LIMIT]} over {FILE_LIMIT}")
    return 0


def cmd_show(files: list[FileInfo], paths: list[str]) -> int:
    by = {f.path: f for f in files}
    missing = 0
    for p in paths:
        f = by.get(p.strip("./") if p.startswith("./") else p)
        if not f:
            print(f"{p}: not measured (missing, generated or outside crates/)")
            missing += 1
            continue
        print(f"{f.path}: {f.code} code, {f.total} total, {f.comment} comment, {f.blank} blank, {f.test_code} in test modules")
    return 1 if missing else 0


def cmd_json(files: list[FileInfo]) -> int:
    print(json.dumps({
        "files": [dict(path=f.path, crate=f.crate, total=f.total, code=f.code, comment=f.comment, blank=f.blank,
                       test_code=f.test_code, signature_allows=f.signature_allows, marker=f.marker,
                       functions=[dict(name=x.name, code=x.code, start=x.start, end=x.end) for x in f.fns if x.code > FN_FLOOR])
                  for f in files],
        "crates": crate_table(files),
        "over_limit": over_limit(files),
        "fn_over_limit": fn_and_allow_over_limit(files),
    }, indent=1))
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["report", "check", "json", "seed", "show"])
    ap.add_argument("paths", nargs="*", help="files for `show` (repository-relative)")
    ap.add_argument("--date", default=datetime.date.today().isoformat())
    ap.add_argument("--out", type=Path)
    ap.add_argument("--root", type=Path, help="measure another tree (tests)")
    a = ap.parse_args(argv)
    global ROOT, KNOWN, FN_KNOWN
    if a.root:
        ROOT = a.root.resolve()
        KNOWN = ROOT / "docs/dev/structure-known.txt"
        FN_KNOWN = ROOT / "docs/dev/function-known.txt"
    files = collect()
    if a.command == "check":
        return cmd_check(files)
    if a.command == "seed":
        return cmd_seed(files)
    if a.command == "show":
        return cmd_show(files, a.paths)
    if a.command == "json":
        return cmd_json(files)
    return cmd_report(files, a.date, a.out)


if __name__ == "__main__":
    sys.exit(main())
