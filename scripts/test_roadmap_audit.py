#!/usr/bin/env python3
"""Tests of the H01 rule of `roadmap_audit.py` (ISS-025): a checked board line must cite a commit
hash that git resolves. Run: python3 scripts/test_roadmap_audit.py"""
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import roadmap_audit as r  # noqa: E402


def head_hash() -> str:
    return subprocess.run(["git", "rev-parse", "--short=10", "HEAD"], cwd=r.ROOT,
                          capture_output=True, text=True, check=True).stdout.strip()


class Slug(unittest.TestCase):
    def test_bold_title_becomes_a_dashed_id(self):
        line = "- [x] 🟠 **Batch 4: Source files to the 150-line rule** (M-0151) — done"
        self.assertEqual(r.slug(line), "batch-4-source-files-to-the-150-line-rule")

    def test_line_without_bold_title_uses_its_start(self):
        self.assertEqual(r.slug("- [x] plain text, no title"), "plain-text-no-title")


class UnhashedBoardLines(unittest.TestCase):
    def test_a_resolved_hash_passes_a_missing_one_fails(self):
        lines = [
            f"- [x] **Done item** — done 2026-11-22, commit `{head_hash()}`.",
            "- [x] **Done without hash** — done 2026-11-22 on agent/x.",
            "- [ ] **Open item** — no hash needed.",
        ]
        found = r.unhashed_board_lines(lines)
        self.assertEqual([key for key, _ in found], ["done-without-hash"])

    def test_an_invented_hash_fails(self):
        lines = ["- [x] **Invented** — commit `0123456789abcdef0123456789abcdef01234567`."]
        self.assertEqual(len(r.unhashed_board_lines(lines)), 1)

    def test_a_hash_of_a_tree_or_blob_is_not_a_commit(self):
        tree = subprocess.run(["git", "rev-parse", "--short=10", "HEAD^{tree}"], cwd=r.ROOT,
                              capture_output=True, text=True, check=True).stdout.strip()
        lines = [f"- [x] **Tree** — hash `{tree}`."]
        self.assertEqual(len(r.unhashed_board_lines(lines)), 1)


if __name__ == "__main__":
    unittest.main()
