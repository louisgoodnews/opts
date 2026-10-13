# Progress: M-0088 (no blocking work on the main thread; Settings -> Content freeze)

Branch `fix/settings-content-freeze`, worktree `/home/louisgoodnews/Dokumente/opts-wt-content-freeze`.
Scratch (deleted afterwards): `~/.local/share/opts/benchmark/content-freeze/` (cargo target, XDG dirs, catalogue, screenshots).
OMSI: `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`.

## Resume
`git status --short; git log --oneline master..HEAD`; read the checklist; continue at the first open step.
Gates: fmt, clippy (-D warnings, all targets/features), test --workspace --all-features, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`, `scripts/check_links.py`, `scripts/gen_roadmap_copy.py --check`.
Final: merge master into branch, re-gate, push, ff-merge into master in the project folder, push, remove worktree (keep branch).

## Checklist
- [x] S1 diagnostic: `state.frame_ms_max` (worst frame of the last 10 s) in the control protocol
- [x] S2 baseline measurement on the full catalogue (679,543 files / 75,313 entries)
- [ ] S3 background provider for catalogue stats / scan inputs / cache usage (cached, invalidated, 'Calculating...')
- [ ] S4 audit of other UI systems and fixes
- [ ] S5 regression guards (slow-provider frame test, source lint) and standards rule
- [ ] S6 live after-measurement
- [ ] S7 docs (M-0088, ROADMAP, BOARD, R-31), gates, merge

## Findings

- Design: `content_facts.rs` (ContentFactsStore, FactsProvider, worker thread + channel, light/heavy facts, `WantsContentFacts` marker, theme file list), info lines `ContentLine` + `fill_content_lines`, scan chip uses facts ("Checking..."), Home hero decoded in IoTaskPool (`home/hero.rs`), drive history loaded at Startup, settings saves via `GameSettings::save_in_background` (shared Baseline Arc), thumbnails probe in tasks (`Probe`), lint `tests/no_blocking_ui.rs`, budget test in `settings_screen/tests.rs`, standards 3.13.
- STATE (after container restart): all of the above written but NOT yet compiled. Next: build base (worktree `~/.local/share/opts/benchmark/content-freeze/base`, detached HEAD 9b06d56) release to measure baseline; compile+fix mine; run tests.
- Baseline build log `.../content-freeze/build-base.log`; target `.../content-freeze/target`.

## Baseline (S2), master + frame_ms_max, release, headless, catalogue of the full OMSI folder (692,725 files / 75,589 entries, scan 10 s here), warm disk
Opening Settings -> Content (3 repetitions): worst frame 257 ms, 350 ms, 213 ms (first open 233 ms); idle frames 17-21 ms.
Run: `opts-base --headless --cli-port 44777 --omsi-path <OMSI>`, XDG dirs under `.../content-freeze/xdg-base`, script: click `shell.nav.settings`, `settings/category/content`, poll `state.frame_ms_max`.
(The project lead sees a longer freeze: cold caches / slower disk / scan running; the blocking calls are the same.)
