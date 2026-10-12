# Progress: Phase 8 stage 1 (visual quality)

Branch `agent/visual-quality`, worktree `/home/louisgoodnews/Dokumente/opts-wt-visual-quality` (from `agent/texture-proposal`).
Bulky data: `~/.local/share/opts/benchmark/dev-visual/`.

## Environment

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-visual-quality
export CARGO_TARGET_DIR=/home/louisgoodnews/Dokumente/opts-wt-premerge/target   # reuse deps; check `pgrep -a cargo` first
```

Gates (before each commit): `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

## Batches

| Batch | Status |
| :--- | :--- |
| V0 Orient + metrics tool | done |
| V1 Sampler | done |
| V2 Mipmaps (uncompressed) | done |
| V3 Looks | done |
| V4 Verification and hand-over | open |

## Baseline gate failures (present on `agent/texture-proposal` before this work; not mine, not fixed)

- `cargo test`: `opts-cli` `args::tests::test_cli_parsing_defaults` (expects 1280x720, code returns 1920x1080); `opts-game` `input_gating_audit::every_input_reading_file_has_been_reviewed`; `opts-game` `post_new_game_loading_tests` (5 tests).
- `RUSTDOCFLAGS=-D warnings cargo doc`: `opts-game` `Baseline` doc links to a private field; unresolved intra-doc links in `opts-sim` (`SplinePath::kind`), `opts-gui` (`NodeBundle`), `opts-bevy` (`SpatialBundle`, `SpotLightBundle`, `PointLightBundle` in `scenery.rs`, `vehicle.rs`).
- Fixed here only because fmt/clippy gates need them: `cargo fmt` of `opts-bevy/src/position_tag.rs`; a `sort_by_key` in `opts-core/examples/ai_model_vars.rs`.
- Gate rule used: no NEW failure relative to this list.

## Plan (V0)

- Sampler: `opts_bevy::texture_filter` (new): `TextureFiltering {Off,X4,X8,X16}`, pure `object_sampler(TextureFiltering)`, a process-wide current value (atomic) set by the game from `GameSettings`; `model::repeat_sampler()` delegates to it, so `ContentLoader::texture`, hot reload and text textures share one function. CLI override `--texture-filter` (run only).
- Mips: `opts_bevy::texture::generate_mip_chain` pure; called from `load_bevy_image` callers (loader, hot reload) and text textures; skipped when filtering is Off.
- Look: `opts-game` `in_game/look.rs` (`Look` enum, `LookParams`, resource, system for `Tonemapping`/`ColorGrading`, ambient scale read by `sky.rs`), settings key `look`, control `render.look`, CLI `--look`.

## Results

- V0: `opts-tools image-metrics` works (reproduces the proposal's trend on its saved screenshots: Laplacian a0_base 0.1637 -> a4_aniso_mips 0.1453, -11 %; saturation unchanged).

- V1: shared sampler (`opts_bevy::texture_filter`), settings keys `texture_filtering` / `look` (absent keys default to 16x / Vivid, test with an old database), Display & Sound tab rows for both, `--texture-filter`/`--look` run-only overrides. Text textures: same sampler and (V2) a generated chain; revert to anisotropy 1 only if signs lose legibility (V4 checks). Tests: `tests/shared_sampler.rs`, `texture_filter` unit tests, settings round trip, `render_choice` tests.

- V2 (mip chains): `generate_mip_chain` (`texture/mips.rs`), DDS re-layout (`texture/dds_chain.rs`). Corpus scan (`examples/texture_scan.rs`, 227,450 image files, 227,371 decoded, 79 failed to decode as before): DDS with a declared chain 35,004, **dropped by the loader 2,727 (7.8 %) -> 0 after the re-layout fix** (cause: block-compressed files whose size is not a multiple of 4; the loader rounds the extent, the file's chain follows the unrounded size). Cost: 3,170 chains of the ADDON_Bad_Huegelsdorf textures in 7.56 s = **2.4 ms per texture** (6.0 GB base data, ~800 MB/s, dev profile, single thread); 1,667 chains of a 6,000-file sample 2.6 ms. First version (float, general path) was 35 ms per texture at 3.5 MB average; an integer 2x2 fast path brought it to 6.7 ms on the same set. Load time Bad_Huegelsdorf_2020 + MAN_SD77 (start to HUD, throw-away config, 3 runs each, quiet machine): filtering Off 10.83/10.51/10.01 s (mean 10.45), 16x 10.98/11.44/10.96 s (mean 11.13): **+0.68 s, +6.5 %**; 230 chains generated in 0.99-1.02 s of that. Data: `~/.local/share/opts/benchmark/dev-visual/shots/lt_*.log`.
- V3 (looks): `look.rs` (Look, LookParams, ActiveLook, LookPlugin, `effective_ambient_scale`), settings keys, settings screen rows, `render.look`, `--look`. Deviation found by looking at night images: ambient x0.5 darkened dusk/night to near black, so the ambient scale **fades to 1 below 25 degrees of sun elevation** (smoothstep), i.e. Vivid changes the ambient only in daylight. Also fixed: `vehicle.get` returned tile-local coordinates although the protocol says absolute (needed by `scripts/visual-ab.sh`).
- Gates after V2+V3: fmt ok, clippy ok, link check 0 broken, roadmap copy up to date; tests and rustdoc fail only in the baseline list above (no new failure).

## Open problems

(none yet)
