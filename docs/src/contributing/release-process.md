# Release Process

> 🚧 Semantic versioning, the changelog and binary publishing are **not yet written**. This page holds the one part that is: the **release gate** of the refactoring taskforce ([Phase 999](../roadmap/phase-999/README.md), [M-0126](../roadmap/phase-999/M-0126.md)).

## 🚦 Release gate: before a `v0.x.0` tag

The last refactoring cycle before a public version ([P-1015](../../dev/prompts/process/P-1015-refactoring-cycle.md)) is the release gate. A tag is cut only when every condition below holds. Each one is a command or a file, not an opinion; the person who runs the gate pastes the command and its output into the cycle report `docs/src/meta/structure-refactoring-<date>.md`.

| # | Condition | How it is checked |
| :---: | :--- | :--- |
| 1 | All gates pass on the release commit | `./scripts/local-ci.sh` ends with `Local CI PASSED` (fmt, clippy, test, doc, mdbook, roadmap, structure) |
| 2 | The structure ratchet is not worse than after the last cycle | `python3 scripts/structure_audit.py check` exits 0 and `git diff <last-cycle-tag> -- docs/dev/structure-known.txt` shows no file added and no size raised |
| 3 | No file over 500 lines of code without a recorded reason | every line of `docs/dev/structure-known.txt` has a `# reason` that is not only the default "planned: ..." text |
| 4 | The OMSI boundary did not widen | `python3 scripts/omsi_uses.py --check` exits 0 and `docs/dev/omsi-uses-known.txt` is not higher than at the last release ([ADR-0014](../adr/0014-data-model-separation.md)) |
| 5 | The public API lost nothing without a note | `cargo public-api diff <last-release>..HEAD` for each library crate (or the manual diff of the `pub use` paths) shows no removed item that is not in the changelog with a migration note |
| 6 | The crate documents match the tree | `python3 scripts/crate_graph.py --check` exits 0 and every crate in `Cargo.toml` has `docs/crates/<crate>.md` |
| 7 | Documentation links hold and the roadmap is in step | `python3 scripts/check_links.py` and `python3 scripts/roadmap_audit.py --check` exit 0 |
| 8 | The cycle report exists | `docs/src/meta/structure-refactoring-<date>.md` is committed, with the before and after table and the reverted refactorings |
| 9 | Nothing is in flight on the files being released | `docs/dev/BOARD.md` has no open item announced as frozen for a cycle batch |

**Who signs off:** the project lead approves the gate (the last checkpoint of the cycle prompt); a session never tags a release by itself. A failed condition is not waived in the commit message: it is fixed, or recorded on the BOARD with the project lead's decision and a date.

Between releases a cycle runs at the end of a development cycle with the same procedure but without the sign-off of conditions 5 and 9 being mandatory.
