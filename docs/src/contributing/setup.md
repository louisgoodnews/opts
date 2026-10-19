# Development Setup

> 🚧 **Not yet written.** Prerequisites, Rust toolchain setup, workspace build instructions, and verification.

## What `scripts/local-ci.sh` checks

`./scripts/local-ci.sh` runs, in order, the gates a change has to pass before it is merged:

| Step | Command | What fails it |
| :--- | :--- | :--- |
| fmt | `cargo fmt --check` | unformatted code |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | any warning |
| test | `cargo test --workspace --all-features` | a failing test (golden screens, corpus tests when `OMSI_PATH` is set) |
| doc | `cargo doc` | a broken doc link or warning |
| mdbook | `mdbook build` + `scripts/check_links.py` | a broken relative link in the documentation |
| roadmap | `gen_roadmap_copy.py`, `roadmap_audit.py`, `crate_graph.py`, `omsi_uses.py` (each with `--check`) | drift between the roadmap, the crate graph and the code; a new reach of an engine crate into OMSI types ([ADR-0014](../adr/0014-data-model-separation.md)) |
| structure | `test_structure_audit.py`, `structure_audit.py check` | a `.rs` file over **500 lines of code** that is not in `docs/dev/structure-known.txt`, or a known one that grew ([standards 3.7](standards.md)) |

The structure step is a **ratchet**: the files already over the limit are listed with their size and a reason, and may only shrink. When it fails it names the file and the measure (`FAIL crates/x/src/y.rs: grew from 612 to 618 lines of code`). Split the file by concern; do not edit the known file to make a failure go away. `python3 scripts/structure_audit.py report` writes the full measurement (files, functions, modules, crates, standards violations) to `docs/src/meta/structure-audit-<date>.md`, and `seed` rewrites the known file after files shrank.

