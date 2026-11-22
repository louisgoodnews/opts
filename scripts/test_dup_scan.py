#!/usr/bin/env python3
"""Tests of `dup_scan.py` on a fixture tree (ISS-035): a block of 10 code lines copied into two
files is one duplicated run; a comment-only difference still matches; a short block is not
reported; a `pub fn` with no other reference is unused, and one referenced from another file is
not. Run: python3 scripts/test_dup_scan.py"""
import contextlib
import io
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import dup_scan as ds  # noqa: E402

BLOCK = "".join(f"    let value_{i} = compute_something_long({i}, \"text {i}\");\n" for i in range(10))


def run(root: Path, *args: str) -> str:
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        ds.main([*args, "--root", str(root)])
    return out.getvalue()


class Fixture(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="dup-scan-")).resolve()
        src = self.root / "crates/demo/src"
        src.mkdir(parents=True)
        (src / "one.rs").write_text(
            "//! One.\nfn first() {\n" + BLOCK + "}\n\npub fn orphan() -> u32 { 1 }\n"
            "pub fn used() -> u32 { 2 }\n"
        )
        # the same block with a comment line in the middle: comments do not change the match
        (src / "two.rs").write_text(
            "//! Two.\nfn second() {\n" + BLOCK.replace("    let value_5", "    // copy\n    let value_5")
            + "}\n\nfn short() {\n    let a = 1;\n}\n"
        )
        # a unique file, and two copies of an 8-line block of short calls (under MIN_CHARS): not reported
        short = "fn short() {\n" + "    x();\n" * 8 + "}\n"
        (src / "three.rs").write_text("//! Three.\nfn third() -> u32 {\n    used()\n}\n" + short)
        (src / "four.rs").write_text("//! Four.\n" + short)

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_a_block_copied_into_two_files_is_one_run_of_ten_lines(self):
        found = ds.clusters(self.root)
        self.assertEqual(len(found), 1)
        c = found[0]
        self.assertEqual({c["a"], c["b"]}, {"crates/demo/src/one.rs", "crates/demo/src/two.rs"})
        # the block's 10 lines and the closing brace after it match: 4 windows of 8, 11 lines
        self.assertEqual(c["lines"], 11)
        self.assertEqual(c["a_line"], 3)  # the first block line of one.rs (line 3, after the fn line)
        self.assertIn("duplicated runs", run(self.root, "dup", "--top", "3"))

    def test_a_short_block_and_a_unique_file_are_not_reported(self):
        found = ds.clusters(self.root)
        short_files = {"crates/demo/src/three.rs", "crates/demo/src/four.rs"}
        self.assertEqual([c for c in found if short_files & {c["a"], c["b"]}], [])

    def test_unused_pub_items_are_listed_and_referenced_ones_are_not(self):
        out = run(self.root, "unused-pub")
        self.assertIn("1 of 2 pub items have no reference", out)
        self.assertIn("fn      orphan", out)
        self.assertNotIn("used", out.split("\n", 1)[1])


if __name__ == "__main__":
    unittest.main()
