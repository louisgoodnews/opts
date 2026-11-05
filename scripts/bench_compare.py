#!/usr/bin/env python3
"""Compares two Criterion runs of the M-0028 benches on one machine and reports regressions.

Criterion keeps every case's estimates under `<criterion dir>/<group>/<case>/<run>/estimates.json`;
`<run>` is `new` (the latest run), `base` (the run before it) or a name given with
`--save-baseline`. For every case present in both runs the typical time is compared (Criterion's
`slope` estimate when it has one, as printed by `cargo bench`, else the mean); a case is a
regression when it is slower by more than the threshold AND the two 95 % confidence intervals do
not overlap. Run both on a quiet machine: under another program's load identical code swung by
up to 53 % (2026-11-05), which this rule cannot tell from a regression.

Not a CI gate: shared runners vary by far more than a regression (M-0028 §Scope). Typical use:

  cargo bench -p opts-benchmark --no-default-features --bench parsers -- --save-baseline main
  # ... change the code ...
  cargo bench -p opts-benchmark --no-default-features --bench parsers
  python3 scripts/bench_compare.py --base main

Exit code 1 when a case regressed, 2 when no case is in both runs.
Usage: python3 scripts/bench_compare.py [--dir DIR] [--base NAME] [--new NAME] [--threshold PCT]
"""
import argparse
import json
import os
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


@dataclass
class Estimate:
    """Typical time of one case in nanoseconds, with its 95 % confidence interval."""
    mean: float
    low: float
    high: float


def default_dir() -> Path:
    target = os.environ.get("CARGO_TARGET_DIR")
    return (Path(target) if target else ROOT / "target") / "criterion"


def load_run(criterion: Path, run: str) -> dict[str, Estimate]:
    """Case id (`group/case`) to its typical time (slope, else mean), for every case that has `run`."""
    cases = {}
    for path in sorted(criterion.glob(f"**/{run}/estimates.json")):
        case = path.parent.parent.relative_to(criterion).as_posix()
        estimates = json.loads(path.read_text())
        typical = estimates.get("slope") or estimates["mean"]
        ci = typical["confidence_interval"]
        cases[case] = Estimate(typical["point_estimate"], ci["lower_bound"], ci["upper_bound"])
    return cases


def change(base: Estimate, new: Estimate) -> float:
    """Relative change of the typical time in percent (positive is slower)."""
    return (new.mean / base.mean - 1.0) * 100.0


def regressed(base: Estimate, new: Estimate, threshold: float) -> bool:
    return change(base, new) > threshold and new.low > base.high


def improved(base: Estimate, new: Estimate, threshold: float) -> bool:
    return change(base, new) < -threshold and new.high < base.low


def fmt_time(ns: float) -> str:
    for unit, scale in (("s", 1e9), ("ms", 1e6), ("µs", 1e3)):
        if ns >= scale:
            return f"{ns / scale:.3g} {unit}"
    return f"{ns:.3g} ns"


def compare(base: dict[str, Estimate], new: dict[str, Estimate], threshold: float) -> tuple[list[str], int]:
    """Report lines and the number of regressions."""
    lines, regressions = [], 0
    for case in sorted(base.keys() & new.keys()):
        b, n = base[case], new[case]
        if regressed(b, n, threshold):
            verdict, regressions = "REGRESSION", regressions + 1
        elif improved(b, n, threshold):
            verdict = "faster"
        else:
            verdict = "same"
        lines.append(f"{case:<40} {fmt_time(b.mean):>10} -> {fmt_time(n.mean):>10} "
                     f"{change(b, n):+7.1f} %  {verdict}")
    for case in sorted(base.keys() - new.keys()):
        lines.append(f"{case:<40} only in the base run")
    for case in sorted(new.keys() - base.keys()):
        lines.append(f"{case:<40} only in the new run")
    return lines, regressions


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dir", type=Path, default=None, help="Criterion directory (default: <target>/criterion)")
    parser.add_argument("--base", default="base", help="run to compare against (default: base)")
    parser.add_argument("--new", default="new", help="run to check (default: new)")
    parser.add_argument("--threshold", type=float, default=5.0, help="slowdown in percent that counts (default: 5)")
    args = parser.parse_args(argv)
    criterion = args.dir or default_dir()
    base, new = load_run(criterion, args.base), load_run(criterion, args.new)
    if not base.keys() & new.keys():
        print(f"no case has both runs '{args.base}' and '{args.new}' under {criterion}")
        return 2
    lines, regressions = compare(base, new, args.threshold)
    print("\n".join(lines))
    print(f"{regressions} regression(s) over {args.threshold:g} % in {len(base.keys() & new.keys())} case(s)")
    return 1 if regressions else 0


if __name__ == "__main__":
    sys.exit(main())
