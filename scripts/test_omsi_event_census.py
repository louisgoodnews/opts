#!/usr/bin/env python3
"""Tests of `omsi_event_census.py` on in-code samples (M-0118): the literal and non-literal event
sites, trigger blocks, the `[trigger]`, `[entry]` and `[mouseevent]` line rules, the
classification counts, and the rule that the table carries no name. No file is read or written.
Run: python3 scripts/test_omsi_event_census.py"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import omsi_event_census as ec  # noqa: E402

SCRIPT = """
(T.L.ev_demo_one)
(t.f.ev_demo_announce)
( T.S.demo_sound )
(T.L.$bad)
(T.L.)
{trigger:demo_key}
{ trigger : demo_mesh }
"""

CFG = "[trigger]\nev_demo_one\n\n[trigger]\n   ev_demo_two  \n"
KEYS = "[entry]\n\ndemo_key\n[entry]\nother_key\n"
MOUSE = "[mouseevent]\ndemo_mesh\n[mouseevent]\n\ndemo_door\n"


class ParserTest(unittest.TestCase):
    def test_event_sites_by_kind_and_case(self):
        sites, dyn, blocks = ec.scan_script(SCRIPT)
        self.assertEqual(sites["L"], ["ev_demo_one"])
        self.assertEqual(sites["F"], ["ev_demo_announce"])
        self.assertEqual(sites["S"], ["demo_sound"])

    def test_non_literal_accessors_are_counted_not_listed(self):
        _, dyn, _ = ec.scan_script(SCRIPT)
        self.assertEqual(dyn, 2)  # "$bad" and the empty name

    def test_trigger_blocks_allow_inner_spaces(self):
        _, _, blocks = ec.scan_script(SCRIPT)
        self.assertEqual(blocks, ["demo_key", "demo_mesh"])

    def test_first_nonblank_line_after_header(self):
        self.assertEqual(ec.first_after(CFG.splitlines(), "trigger"), ["ev_demo_one", "ev_demo_two"])
        self.assertEqual(ec.first_after(KEYS.splitlines(), "entry"), ["demo_key", "other_key"])
        self.assertEqual(ec.first_after(MOUSE.splitlines(), "mouseevent"), ["demo_mesh", "demo_door"])

    def test_header_without_following_line_gives_nothing(self):
        self.assertEqual(ec.first_after(["[trigger]", "  "], "trigger"), [])


class ClassifyTest(unittest.TestCase):
    def census(self):
        return {"files": 3, "dynamic": 0, "L": ["ev_a", "ev_b"], "F": ["ev_f"], "S": ["snd"],
                "sound": ["ev_a", "ev_c", "ev_c"], "keys": ["key_x"], "mouse": ["mesh_m"],
                "blocks": ["ev_a", "key_x", "key_x_off", "mesh_m_drag", "loose", "loose"]}

    def test_counts(self):
        rows = dict(ec.classify(self.census()))
        self.assertEqual(rows["raised names (T.L)"], 2)
        self.assertEqual(rows["  raised and bound by a sound trigger"], 1)
        self.assertEqual(rows["  raised, no sound binding"], 1)
        self.assertEqual(rows["payload names (T.F or T.S)"], 2)
        self.assertEqual(rows["sound names never raised as T.L"], 1)
        self.assertEqual(rows["trigger-block names"], 5)
        self.assertEqual(rows["residual names (engine-raised candidates)"], 1)
        self.assertEqual(rows["residual block sites"], 2)

    def test_rendered_table_has_no_event_name(self):
        c = self.census()
        text = ec.render(c, ec.classify(c))
        for name in ("ev_a", "ev_b", "ev_c", "ev_f", "key_x", "mesh_m", "loose", "snd"):
            self.assertNotIn(name, text)
        self.assertIn("| raised names (T.L) | 2 |", text)

    def test_identifiers_are_sorted_distinct_and_pattern_checked(self):
        c = self.census()
        c["blocks"].append("bad name")  # space: left out and counted
        c["mouse"].append("quo'te")  # quote: left out and counted
        kept, left_out = ec.identifiers(c)
        self.assertEqual(kept, sorted(set(kept)))
        for name in kept:
            self.assertRegex(name, r"^[a-z0-9_]+$")
        for name in ("ev_a", "ev_b", "ev_c", "ev_f", "snd", "key_x", "mesh_m", "loose"):
            self.assertIn(name, kept)
        self.assertNotIn("bad name", kept)
        self.assertEqual(left_out, 2)

    def test_default_output_is_counts_only_and_names_are_opt_in(self):
        # Default path: render() prints the count tables only. The identifier list exists only
        # behind --names-out, so every default line is a "| label | number |" row.
        c = self.census()
        text = ec.render(c, ec.classify(c))
        for line in text.splitlines():
            if line.startswith("|") and ":---" not in line and "| Count" not in line \
                    and "| Classification" not in line:
                self.assertRegex(line, r"^\| [^|]+ \| [\d,]+ \|$")


if __name__ == "__main__":
    unittest.main()
