#!/usr/bin/env python3
"""Ratchet for the boundary of ADR-0014: engine crates must not reach OMSI-shaped types.

`opts-omsi` (the legacy readers, also reachable through the temporary facade `opts-core`) is a leaf
adapter. The crates below are the engine side; they still import the OMSI structures in some
files while their consumers are migrated to the native model (`opts-model`). This script counts,
per crate, the `.rs` files that name `opts_omsi` or the facade's `opts_core::formats` /
`opts_core::setvars`, and compares with `docs/dev/omsi-uses-known.txt`:

  * a count above the known one fails (a new reach into OMSI);
  * a count below passes and says so, so the file can be lowered (`--write`);
  * the composition root `opts-game`, the importer and the dev tools are not counted.

Usage: python3 scripts/omsi_uses.py [--check | --write]
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KNOWN = ROOT / "docs/dev/omsi-uses-known.txt"
ENGINE = ["opts-bevy", "opts-sim", "opts-traffic", "opts-gui", "opts-catalogue"]
PATTERN = re.compile(r"\bopts_omsi\b|\bopts_core::(?:formats|setvars)\b|\bopts_core::\{[^}]*\b(?:formats|setvars)\b")


def counts() -> dict[str, int]:
    out = {}
    for crate in ENGINE:
        n = 0
        for p in (ROOT / "crates" / crate).rglob("*.rs"):
            if "target" in p.parts:
                continue
            if PATTERN.search(p.read_text(encoding="utf-8", errors="ignore")):
                n += 1
        out[crate] = n
    return out


def known() -> dict[str, int]:
    out = {}
    if KNOWN.exists():
        for line in KNOWN.read_text(encoding="utf-8").splitlines():
            line = line.split("#")[0].strip()
            if line:
                k, v = line.split()
                out[k] = int(v)
    return out


def main() -> int:
    now = counts()
    if "--write" in sys.argv:
        body = "".join(f"{k} {v}\n" for k, v in sorted(now.items()))
        KNOWN.write_text(
            "# Files of each engine crate that still name opts_omsi or opts_core::formats/setvars\n"
            "# (ADR-0014). The numbers may only fall; regenerate with scripts/omsi_uses.py --write.\n" + body,
            encoding="utf-8",
        )
        print(f"wrote {KNOWN.relative_to(ROOT)}")
        return 0
    base = known()
    bad = 0
    for crate in ENGINE:
        have, allowed = now[crate], base.get(crate, 0)
        if have > allowed:
            print(f"FAIL {crate}: {have} files reach OMSI types, known {allowed} (ADR-0014: engine crates use opts-model)")
            bad += 1
        elif have < allowed:
            print(f"ok   {crate}: {have} (known {allowed}: lower the ratchet with --write)")
        else:
            print(f"ok   {crate}: {have}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
