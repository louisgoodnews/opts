#!/usr/bin/env python3
"""Roadmap drift audit (M-0074): do ROADMAP.md, the phase pages, the milestone files, the board
and the field report agree with each other and with the repository?

Checks (each finding has a code):

  M01  a milestone file's name, its `Milestone ID` row and its heading name the same id
  M02  the milestone's `Phase` row names the folder it lives in
  M03  the milestone's status starts with a word of the status vocabulary
  P01  the milestone is listed (linked) in its phase page's milestone list
  R01  the milestone has a checklist entry in ROADMAP.md
  R02  that checklist entry is ticked exactly when the milestone is complete
  P02  the phase page's list line gives the same state (done / open) as the milestone
  B01  an open milestone appears on the board
  B02  a completed milestone has no open (`[ ]` or `[~]`) board entry of its own
  B03  a board entry says its branch is "not merged" but the branch is in master
  F01  every field-report row names a milestone

`--check` exits 1 for any finding that is not explained in `docs/dev/roadmap-audit-known.txt`
(`CODE target: reason`, one per line); a stale explanation fails too. Without `--check` it prints
every finding and the counts.

Usage: python3 scripts/roadmap_audit.py [--check] [--json]
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PHASES = ROOT / "docs/src/roadmap"
ROADMAP = ROOT / "ROADMAP.md"
BOARD = ROOT / "docs/dev/BOARD.md"
FIELD_REPORTS = sorted((ROOT / "docs/src/quality").glob("field-report-*.md"))
KNOWN = ROOT / "docs/dev/roadmap-audit-known.txt"

# The first word of a status, after its emoji, decides the state.
DONE_WORDS = ("complete", "completed", "done", "implemented")
OPEN_WORDS = ("planned", "proposed", "in", "partly", "partial", "engineering", "written", "merged",
              "core", "not", "started", "decided", "design", "approved", "first", "needs",
              "blocked", "research", "implementation", "animation", "walking", "traffic",
              "panics", "reported", "unverified")
SPECIAL = ("deferred", "obsolete", "superseded", "cancelled", "withdrawn")
EMOJI = "🟢🟡📝⏳⏸⛔🔴🔬✅🟠"


class Milestone:
    def __init__(self, path: Path):
        self.path = path
        self.id = path.stem
        self.folder = path.parent.name
        text = path.read_text(encoding="utf-8")
        self.text = text
        self.heading = next((l for l in text.splitlines() if l.startswith("# ")), "")
        row = lambda key: next((l.split("|")[2].strip() for l in text.splitlines()
                                if l.startswith(f"| **{key}**")), "")
        self.id_row = row("Milestone ID")
        self.phase_row = row("Phase")
        self.status = row("Status")
        self.priority = row("Priority")

    @property
    def state(self) -> str:
        """`done`, `open`, `special` or `unknown`: the leading emoji decides (🟢 and ✅ are done,
        🟡 📝 ⏳ 🔴 🟠 open, ⏸ and ⛔ special); without one the first word does."""
        status = self.status.strip()
        if status[:1] in ("🟢", "✅"):
            return "done"
        if status[:1] in ("🟡", "📝", "⏳", "🔴", "🟠", "🔬"):
            return "open"
        if status[:1] in ("⏸", "⛔"):
            return "special"
        words = re.sub(r"^[^\w]+", "", status).lower().split()
        first = words[0].strip(".,;:()*") if words else ""
        if first in DONE_WORDS:
            return "done"
        if first in SPECIAL:
            return "special"
        if first in OPEN_WORDS:
            return "open"
        return "unknown"


def milestones() -> list[Milestone]:
    return [Milestone(p) for p in sorted(PHASES.glob("phase-*/M-*.md"))]


def line_of(path: Path, needle: str) -> int:
    for i, l in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if needle in l:
            return i
    return 1


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout


def audit() -> list[tuple[str, str, str]]:
    """Findings as `(code, target, message)`."""
    out: list[tuple[str, str, str]] = []
    ms = milestones()
    by_id = {m.id: m for m in ms}
    roadmap = ROADMAP.read_text(encoding="utf-8")
    board = BOARD.read_text(encoding="utf-8")
    board_lines = board.splitlines()

    # roadmap checklist entries: any checklist line that names the milestone (link or text);
    # `own` are the top-level lines (no indent) that link to the milestone's own file: its entry,
    # not a sub-task that mentions it.
    entries: dict[str, list[str]] = {}
    own: dict[str, list[str]] = {}
    for l in roadmap.splitlines():
        m = re.match(r"(\s*)- \[([ xX~])\] (.*)", l)
        if m:
            for mid in set(re.findall(r"\bM-\d{4}\b", m.group(3))):
                entries.setdefault(mid, []).append(m.group(2).lower())
            if not m.group(1):
                for mid in set(re.findall(r"/(M-\d{4})\.md\)", m.group(3))):
                    own.setdefault(mid, []).append(m.group(2).lower())

    for m in ms:
        t = m.id
        if m.id_row != m.id or m.id not in m.heading:
            out.append(("M01", t, f"id row {m.id_row!r}, heading {m.heading!r}"))
        num = re.search(r"Phase (\d+)", m.phase_row)
        folder_num = re.search(r"phase-(\d+)", m.folder)
        if not num or not folder_num or num.group(1) != folder_num.group(1):
            out.append(("M02", t, f"in {m.folder} but the Phase row says {m.phase_row!r}"))
        if m.state == "unknown":
            out.append(("M03", t, f"status vocabulary: {m.status[:60]!r}"))
        # phase page list
        readme = PHASES / m.folder / "README.md"
        lines = [l for l in readme.read_text(encoding="utf-8").splitlines() if f"({m.id}.md)" in l]
        # A line that gives the milestone with its state: `- [M-… — title](M-….md) — state` or a
        # numbered one; a table row or a checklist entry only counts as being listed.
        listed = [l for l in lines if re.match(r"\s*(- |\d+\. )\[M-\d{4}[^\]]*\]\(", l)]
        if not lines:
            out.append(("P01", t, f"not listed in {m.folder}/README.md"))
        # roadmap checklist
        boxes = entries.get(m.id, [])
        if not boxes:
            out.append(("R01", t, "no checklist entry in ROADMAP.md"))
        own_boxes = own.get(m.id, [])
        if own_boxes:
            ticked = all(b == "x" for b in own_boxes)
            if m.state == "done" and not ticked:
                out.append(("R02", t, f"complete but its ROADMAP.md entry is {own_boxes}"))
            if m.state == "open" and any(b == "x" for b in own_boxes):
                out.append(("R02", t, f"open ({m.status[:40]!r}) but ticked in ROADMAP.md"))
        # phase page line state
        for l in listed:
            says_done = bool(re.search(r"🟢|✅|[Cc]omplete", l.split("](")[-1]))
            says_open = bool(re.search(r"📝|⏳|🟡|[Pp]lanned|[Ii]n progress", l.split("](")[-1]))
            if m.state == "done" and says_open and not says_done:
                out.append(("P02", t, f"complete but the phase page says: {l.strip()[:90]}"))
            if m.state == "open" and says_done and not says_open:
                out.append(("P02", t, f"open but the phase page says: {l.strip()[:90]}"))
        # board
        mentions = [l for l in board_lines if re.search(rf"\b{m.id}\b", l)]
        if m.state == "open" and not mentions:
            out.append(("B01", t, "open milestone without any board entry"))
        if m.state == "done":
            own_open = [l for l in mentions
                        if re.match(r"\s*- (\[[ ~]\]|`\[[ ~]\]`)", l)
                        and re.search(rf"\((?:{m.id}[^)]*)\)", l)
                        and "addendum" not in l.lower()
                        and len(set(re.findall(r"M-\d{4}", l.split("—")[0]))) == 1]
            for l in own_open:
                out.append(("B02", t, f"complete but open on the board: {l.strip()[:90]}"))

    # a line that says a branch is not merged although it is (or that the branch is gone)
    merged = {b.strip().lstrip("* ") for b in git("branch", "-a", "--merged", "master").splitlines()}
    sources = [(ROADMAP, ROADMAP.read_text(encoding="utf-8").splitlines()),
               (BOARD, board_lines)]
    sources += [(m.path, [l for l in m.text.splitlines() if l.startswith("| **Status**")]) for m in ms]
    sources += [(p, p.read_text(encoding="utf-8").splitlines()) for p in sorted(PHASES.glob("phase-*/README.md"))]
    for path, lines in sources:
        for i, l in enumerate(lines, 1):
            for br in re.findall(r"`([\w./-]+)`[^|\n]{0,40}?\(?[^)\n]{0,40}\bnot merged", l):
                if not br.startswith(("feat/", "fix/", "agent/", "research/", "docs/")):
                    continue
                name = path.name if path.name != "README.md" else f"{path.parent.name}/README.md"
                where = f"{name}:{i}" if path in (BOARD, ROADMAP) or path.name == "README.md" else name
                if br in merged or f"remotes/origin/{br}" in merged:
                    out.append(("B03", where, f"branch {br} is in master but the text says not merged"))

    # field report rows
    for fr in FIELD_REPORTS:
        for i, l in enumerate(fr.read_text(encoding="utf-8").splitlines(), 1):
            m = re.match(r"\|\s*(R-\d+)\s*\|", l)
            if m and not re.search(r"M-\d{4}", l):
                out.append(("F01", f"{fr.name}:{m.group(1)}", "row names no milestone"))
    return out


def known() -> dict[tuple[str, str], str]:
    rules = {}
    if KNOWN.exists():
        for l in KNOWN.read_text(encoding="utf-8").splitlines():
            if l.strip() and not l.startswith("#"):
                head, _, why = l.partition(": ")
                code, _, target = head.strip().partition(" ")
                rules[(code, target.strip())] = why.strip()
    return rules


def main() -> int:
    findings = audit()
    ms = milestones()
    counts: dict[str, int] = {}
    for c, _, _ in findings:
        counts[c] = counts.get(c, 0) + 1
    rules = known()
    unexplained = [f for f in findings if (f[0], f[1]) not in rules]
    stale = [k for k in rules if (k[0], k[1]) not in {(c, t) for c, t, _ in findings}]
    if "--json" in sys.argv:
        print(json.dumps({"milestones": len(ms), "findings": findings, "counts": counts}, indent=1))
        return 0
    states = {}
    for m in ms:
        states[m.state] = states.get(m.state, 0) + 1
    print(f"{len(ms)} milestones ({states}); {len(findings)} findings, {len(findings) - len(unexplained)} explained")
    for code in sorted(counts):
        print(f"  {code}: {counts[code]}")
    for c, t, msg in (unexplained if "--check" in sys.argv else findings):
        print(f"{c} {t}: {msg}")
    for code, target in stale:
        print(f"stale explanation: {code} {target}")
    if "--check" in sys.argv and (unexplained or stale):
        print("roadmap audit: unexplained mismatches (fix the files or explain them in "
              "docs/dev/roadmap-audit-known.txt)", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
