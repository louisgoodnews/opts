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
| V1 Sampler | open |
| V2 Mipmaps (uncompressed) | open |
| V3 Looks | open |
| V4 Verification and hand-over | open |

## Baseline gate failures (present on `agent/texture-proposal` before this work; not mine, not fixed)

- `cargo test`: `opts-cli` `args::tests::test_cli_parsing_defaults` (expects 1280x720, code returns 1920x1080); `opts-game` `input_gating_audit::every_input_reading_file_has_been_reviewed`; `opts-game` `post_new_game_loading_tests` (5 tests).
- `RUSTDOCFLAGS=-D warnings cargo doc`: unresolved intra-doc links in `opts-sim` (`SplinePath::kind`), `opts-gui` (`NodeBundle`), `opts-bevy` (`SpatialBundle`, `SpotLightBundle`, `PointLightBundle` in `scenery.rs`, `vehicle.rs`).
- Fixed here only because fmt/clippy gates need them: `cargo fmt` of `opts-bevy/src/position_tag.rs`; a `sort_by_key` in `opts-core/examples/ai_model_vars.rs`.
- Gate rule used: no NEW failure relative to this list.

## Plan (V0)

- Sampler: `opts_bevy::texture_filter` (new): `TextureFiltering {Off,X4,X8,X16}`, pure `object_sampler(TextureFiltering)`, a process-wide current value (atomic) set by the game from `GameSettings`; `model::repeat_sampler()` delegates to it, so `ContentLoader::texture`, hot reload and text textures share one function. CLI override `--texture-filter` (run only).
- Mips: `opts_bevy::texture::generate_mip_chain` pure; called from `load_bevy_image` callers (loader, hot reload) and text textures; skipped when filtering is Off.
- Look: `opts-game` `in_game/look.rs` (`Look` enum, `LookParams`, resource, system for `Tonemapping`/`ColorGrading`, ambient scale read by `sky.rs`), settings key `look`, control `render.look`, CLI `--look`.

## Results

- V0: `opts-tools image-metrics` works (reproduces the proposal's trend on its saved screenshots: Laplacian a0_base 0.1637 -> a4_aniso_mips 0.1453, -11 %; saturation unchanged).

## Open problems

(none yet)
