#!/usr/bin/env python3
"""Tests of `structure_audit.py` on a fixture tree (M-0125): a 600-line file, a 100-line function,
a single-file module, and the ratchet (`check` fails when a known file grows, passes when it
shrinks). Run: python3 scripts/test_structure_audit.py"""
import contextlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import structure_audit as sa  # noqa: E402



def run(root: Path, *args: str) -> tuple[int, str]:
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = sa.main([*args, "--root", str(root)])
    return code, out.getvalue()


def body(lines: int) -> str:
    return "".join(f"    let v{i} = {i};\n" for i in range(lines))


class Fixture(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix="structure-audit-")).resolve()
        src = self.root / "crates/demo/src"
        (src / "single").mkdir(parents=True)
        (self.root / "docs/dev").mkdir(parents=True)
        (self.root / "docs/src/meta").mkdir(parents=True)
        (self.root / "crates/demo/Cargo.toml").write_text('[package]\nname = "demo"\n[dependencies]\nserde = "1"\nopts-types = { path = "x" }\n')
        # lib.rs: documented, 600 lines of code (one function of 590) plus comments and blanks.
        (src / "lib.rs").write_text("//! Demo.\n\n// a comment\n/// doc\npub fn big() {\n" + body(590) + "}\n\n/* block\n comment */\n" + "".join(f"pub const C{i}: u32 = {i};\n" for i in range(5)))
        # a 100-line function in a small file, an undocumented pub item, an unjustified allow, an unwrap
        (src / "small.rs").write_text("//! Small.\n\npub fn hundred() {\n" + body(100) + "}\n\npub fn bare() {}\n\n#[allow(dead_code)]\nfn x() { Some(1).unwrap(); }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { Some(1).unwrap(); }\n}\n")
        # a single-file module directory, and a file with no module doc
        (src / "single/mod.rs").write_text("pub fn only() {}\n")

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def test_the_measurements_are_exactly_what_the_fixture_holds(self):
        code, out = run(self.root, "json")
        data = json.loads(out)
        files = {f["path"]: f for f in data["files"]}
        lib = files["crates/demo/src/lib.rs"]
        # `pub fn big() {` + 590 body lines + `}` + 5 consts = 597 lines of code; the doc comment, the
        # line comment, the block comment (2 lines) and the leading `//!` are comments.
        self.assertEqual(lib["code"], 597)
        self.assertEqual(lib["comment"], 5)
        self.assertEqual(files["crates/demo/src/small.rs"]["functions"][0]["name"], "hundred")
        self.assertEqual(files["crates/demo/src/small.rs"]["functions"][0]["code"], 102)  # signature + 100 + closing brace
        self.assertEqual(data["over_limit"], {"crates/demo/src/lib.rs": 597})
        self.assertEqual([f["name"] for f in lib["functions"]], ["big"])

    def test_the_report_names_what_the_fixture_has(self):
        out = self.root / "report.md"
        code, _ = run(self.root, "report", "--date", "2000-01-01", "--out", str(out))
        self.assertEqual(code, 0)
        text = out.read_text()
        self.assertIn("`crates/demo/src/lib.rs`", text)
        self.assertIn("`hundred`", text)
        self.assertIn("over 80", text)
        self.assertIn("Single-file module directories: `crates/demo/src/single`", text)
        # one undocumented pub item (`bare`), one unjustified allow, one unwrap outside tests, one file without //!
        row = {l.split("|")[1].strip(): l.split("|")[2].strip() for l in text.splitlines() if l.startswith("| ") and "count" not in l and len(l.split("|")) > 3}
        self.assertEqual(row["`#[allow]` without a justification comment"], "1")
        self.assertEqual(row["`unwrap()` / `expect(` in library paths"], "1")
        self.assertEqual(row["module file without a `//!` document (3.8)"], "1")

    def test_the_ratchet_fails_on_growth_and_new_files_and_passes_on_shrinkage(self):
        known = self.root / "docs/dev/structure-known.txt"
        self.assertEqual(run(self.root, "seed")[0], 0)
        self.assertIn("crates/demo/src/lib.rs 597", known.read_text())
        self.assertEqual(run(self.root, "check")[0], 0)
        lib = self.root / "crates/demo/src/lib.rs"
        original = lib.read_text()
        # growth of a known file
        lib.write_text(original + "pub const MORE: u32 = 1;\n")
        code, out = run(self.root, "check")
        self.assertEqual(code, 1)
        self.assertIn("crates/demo/src/lib.rs: grew from 597 to 598", out)
        # shrinkage passes and says so
        lib.write_text(original.replace("    let v0 = 0;\n", ""))
        code, out = run(self.root, "check")
        self.assertEqual(code, 0)
        self.assertIn("known 597", out)
        # a new file over the limit
        lib.write_text(original)
        (self.root / "crates/demo/src/new.rs").write_text("//! New.\n" + "".join(f"pub const N{i}: u32 = {i};\n" for i in range(501)))
        code, out = run(self.root, "check")
        self.assertEqual(code, 1)
        self.assertIn("crates/demo/src/new.rs: 501 lines of code", out)

    def test_a_file_just_under_the_limit_is_fine_and_the_501st_line_is_not(self):
        (self.root / "crates/demo/src/edge.rs").write_text("//! Edge.\n" + "".join(f"pub const E{i}: u32 = {i};\n" for i in range(500)))
        run(self.root, "seed")
        self.assertNotIn("edge.rs", (self.root / "docs/dev/structure-known.txt").read_text())
        (self.root / "crates/demo/src/edge.rs").write_text("//! Edge.\n" + "".join(f"pub const E{i}: u32 = {i};\n" for i in range(501)))
        self.assertEqual(run(self.root, "check")[0], 1)

    def test_generated_and_table_markers(self):
        gen = "//! structure-audit: generated\n" + "".join(f"pub const G{i}: u32 = {i};\n" for i in range(900))
        (self.root / "crates/demo/src/gen.rs").write_text(gen)
        tab = "//! structure-audit: table\n" + "".join(f"pub const T{i}: u32 = {i};\n" for i in range(900))
        (self.root / "crates/demo/src/table.rs").write_text(tab)
        data = json.loads(run(self.root, "json")[1])
        paths = {f["path"] for f in data["files"]}
        self.assertNotIn("crates/demo/src/gen.rs", paths)
        self.assertIn("crates/demo/src/table.rs", paths)
        self.assertNotIn("crates/demo/src/table.rs", data["over_limit"])

    def test_strings_and_comments_do_not_confuse_the_function_scan(self):
        tricky = '//! Tricky.\n\npub fn tricky() {\n    let a = "}{ fn fake() {";\n    let b = \'{\';\n    let r = r#"} } }"#;\n    // fn commented() {\n    let l: &\'static str = "x";\n}\n\npub fn after() {}\n'
        (self.root / "crates/demo/src/tricky.rs").write_text(tricky)
        data = json.loads(run(self.root, "json")[1])
        self.assertTrue(all(not f["functions"] for f in data["files"] if f["path"].endswith("tricky.rs")))  # all under 60
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        info = sa.measure(self.root / "crates/demo/src/tricky.rs")
        self.assertEqual([f.name for f in info.fns], ["tricky", "after"])
        self.assertFalse(info.unbalanced)


if __name__ == "__main__":
    unittest.main()
