#!/usr/bin/env python3
"""Tests of `gen_roadmap_copy.py` (phase section parsing). Regression: a phase heading that
directly follows another phase heading (no body or `---` in between) silently dropped the
earlier phase. Run: python3 scripts/test_gen_roadmap_copy.py"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gen_roadmap_copy as g  # noqa: E402


class PhaseSections(unittest.TestCase):
    def test_adjacent_phase_headings_both_present(self):
        roadmap = "\n".join([
            "## 🧩 Phase 10: Sessions",
            "*Goal: first.*",
            "",
            "- [ ] item ten",
            "",
            "## 🔭 Phase 11: Advanced Systems",
            "*Goal: second.*",
            "",
            "- [ ] item eleven",
            "",
            "---",
            "",
        ])
        sections = g.phase_sections(roadmap)
        self.assertIn(10, sections)
        self.assertIn(11, sections)
        self.assertIn("- [ ] item ten", sections[10])
        self.assertNotIn("- [ ] item eleven", sections[10])
        self.assertIn("- [ ] item eleven", sections[11])

    def test_empty_adjacent_phases_kept(self):
        roadmap = "## Phase 10: A\n## Phase 11: B\n## Phase 12: C\n---\n"
        sections = g.phase_sections(roadmap)
        self.assertEqual({10: [], 11: [], 12: []}, sections)

    def test_non_phase_heading_closes_phase(self):
        roadmap = "## Phase 3: X\nbody\n## Deferred Cleanup\nnot a phase\n"
        sections = g.phase_sections(roadmap)
        self.assertEqual({3: ["body"]}, sections)


if __name__ == "__main__":
    unittest.main()
