#!/usr/bin/env python3
"""Tests of `bench_compare.py` on a fixture Criterion directory (M-0028): a clear slowdown is a
regression, a slowdown inside the noise is not, a speed-up is reported as faster, the slope is
preferred over the mean, and cases in only one run are listed without failing.
Run: python3 scripts/test_bench_compare.py"""
import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import bench_compare as bc  # noqa: E402


def write(root: Path, case: str, run: str, mean: float, half_width: float) -> None:
    path = root / case / run / "estimates.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    interval = {"confidence_level": 0.95, "lower_bound": mean - half_width, "upper_bound": mean + half_width}
    path.write_text(json.dumps({"mean": {"point_estimate": mean, "confidence_interval": interval}}))


def run(root: Path, *args: str) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = bc.main(["--dir", str(root), *args])
    return code, out.getvalue()


class Compare(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_a_clear_slowdown_is_a_regression(self) -> None:
        write(self.root, "tile/500_objects", "base", 1000.0, 10.0)
        write(self.root, "tile/500_objects", "new", 1200.0, 10.0)
        code, out = run(self.root)
        self.assertEqual(code, 1)
        self.assertIn("REGRESSION", out)

    def test_a_slowdown_inside_the_noise_is_not(self) -> None:
        write(self.root, "tile/500_objects", "base", 1000.0, 100.0)
        write(self.root, "tile/500_objects", "new", 1080.0, 100.0)
        code, out = run(self.root)
        self.assertEqual(code, 0)
        self.assertIn("same", out)

    def test_a_slowdown_below_the_threshold_is_not(self) -> None:
        write(self.root, "tile/500_objects", "base", 1000.0, 1.0)
        write(self.root, "tile/500_objects", "new", 1030.0, 1.0)
        self.assertEqual(run(self.root)[0], 0)
        self.assertEqual(run(self.root, "--threshold", "2")[0], 1)

    def test_a_speed_up_is_faster(self) -> None:
        write(self.root, "vehicle_step/at_rest", "base", 200.0, 2.0)
        write(self.root, "vehicle_step/at_rest", "new", 150.0, 2.0)
        code, out = run(self.root)
        self.assertEqual(code, 0)
        self.assertIn("faster", out)

    def test_named_baselines_and_one_sided_cases(self) -> None:
        write(self.root, "tile/500_objects", "main", 1000.0, 10.0)
        write(self.root, "tile/500_objects", "new", 1000.0, 10.0)
        write(self.root, "tile/5000_objects", "main", 9000.0, 10.0)
        write(self.root, "global_cfg/100_tiles", "new", 20.0, 1.0)
        code, out = run(self.root, "--base", "main")
        self.assertEqual(code, 0)
        self.assertIn("tile/5000_objects", out)
        self.assertIn("only in the base run", out)
        self.assertIn("only in the new run", out)

    def test_the_slope_is_used_when_present(self) -> None:
        write(self.root, "tile/500_objects", "base", 1000.0, 10.0)
        write(self.root, "tile/500_objects", "new", 1000.0, 10.0)
        path = self.root / "tile/500_objects/new/estimates.json"
        data = json.loads(path.read_text())
        interval = {"confidence_level": 0.95, "lower_bound": 1490.0, "upper_bound": 1510.0}
        data["slope"] = {"point_estimate": 1500.0, "confidence_interval": interval}
        path.write_text(json.dumps(data))
        self.assertEqual(run(self.root)[0], 1)

    def test_no_common_case_exits_2(self) -> None:
        write(self.root, "tile/500_objects", "base", 1000.0, 10.0)
        self.assertEqual(run(self.root)[0], 2)


if __name__ == "__main__":
    unittest.main()
