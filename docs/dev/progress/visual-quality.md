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
| V4 Verification and hand-over | done |

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

- V4 (verification), screenshots `~/.local/share/opts/benchmark/dev-visual/shots/` (not in the repository), made with `scripts/visual-ab.sh` (deterministic: paused simulation, fixed poses; a second run of the same build reproduces the metrics within 0.03 %, acceptance +-1 %). Metrics on the crop `0,0,1920,860` (HUD strip cut), five poses per configuration (`driver`, `road_grazing`, `foliage`, `far`, `sign_close`):

  | Comparison (same frames) | Result, mean of the 5 poses (per pose in brackets) |
  | :--- | :--- |
  | Filtering Off -> 16x, Bad Hügelsdorf noon, high-frequency energy (Laplacian) | **-17.2 %** (-15.5, -18.8, -15.3, -21.7, -14.4) |
  | Filtering Off -> 16x, Grundorf noon | **-27.9 %** (-15.8, -27.9, -18.3, -44.8, -32.4) |
  | Vivid vs Bevy default, Bad Hügelsdorf noon | saturation **+46.8 %**, luma std (contrast) **+19.8 %**, mean luma -5.9 %, darkest pixel -39 % (0.108 -> 0.066) |
  | Vivid vs Bevy default, Grundorf noon | saturation +50.3 %, contrast +19.0 %, mean luma -0.6 % |
  | Vivid vs Bevy default, Bad Hügelsdorf 21:00 | saturation +9.4 %, mean luma **-11.6 %**, darkest pixel -20 % (0.009 -> 0.005, both near black) |
  | Vivid vs Bevy default, Bad Hügelsdorf 19:00 (dusk, 2 poses) | mean luma -8 to -9 %, saturation +12 % |
  | Natural vs Bevy default | identical (same parameters until the calibration) |

  What I saw (I looked at the images): **Filtering** removes the blocky, pixelated foliage, glass and kerb textures of the Off setting (clearly visible in `road_grazing`), asphalt is calmer at the grazing angle, tree and hedge cutouts keep their silhouettes (no visible thinning at the distance of the `far` pose, thanks to the cutout coverage rule), text on the VBBH stop sign and the timetable stays readable. No shimmer test in motion was possible (stills only): that is for the project lead's test. **Vivid** gives clearly richer greens, a deeper blue sky and more contrast; it is a little darker overall, shadows are deeper (this is the intended trade-off of the proposal). **Night/dusk:** with the ambient factor fading out below 25 degrees of sun elevation the scene stays readable, but Vivid is still 8-12 % darker than Bevy default because the tonemapper and post-saturation differ; at 21:00 both are almost black (the bus's headlights are off at the start of a session, so they could not be compared; the stop sign of the night driver pose is just visible in both). I regard the night result as acceptable, not as a pass: see open problems.

## Test guide for the project lead

Build and run from the worktree (this uses your real settings; that is what you want to test; the first change in the settings screen writes the new `texture_filtering` and `look` keys, an existing `settings.db` without them defaults to 16x and Vivid):

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-visual-quality
cargo run -p opts-game --release -- \
  --omsi-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2" \
  --map-path  "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/maps/Bad_Huegelsdorf_2020" \
  --vehicle-path "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/Vehicles/MAN_SD200/MAN_SD77.bus" \
  --autostart --camera ego --cli-port 44201
```

1. **Look (live):** in another terminal `cargo run -q -p opts-cli -- render.look natural` (also `vivid`, `bevy`; without an argument it prints the current one). Compare `vivid` and `bevy` on the same view; try Left Alt vehicle menu / time of day to see dusk and night. The choice is saved in the settings.
2. **Settings screen:** Main menu -> Settings -> tab *Display & Sound*: two rows of buttons, *Texture filtering* (Off / 4x / 8x / 16x (default)) and *Colour look* (Vivid (default) / Natural / Bevy default). The look applies at once; the filtering applies to textures loaded **afterwards** (start a new session, or edit/touch a texture file to see the hot-reload path).
3. **One-run overrides without touching the settings:** `--texture-filter off|4|8|16` and `--look vivid|natural|bevy` on the game command line.
4. **What to judge:** blocky or shimmering textures while driving (Off vs 16x), hedges/trees at a distance (do they thin out or flicker?), asphalt at grazing angles, signs and bus-stop text, the Vivid grade (too saturated? too dark in shadow?), dusk and night, load time (the log line `generated mip chains {textures, millis}` appears when the HUD comes up; I measured +0.7 s on Bad Hügelsdorf).
5. **Reproduce my numbers:** `scripts/visual-ab.sh` and `opts-tools image-metrics`, see [Visual A/B](../../src/architecture/visual-ab.md). `cargo run --release -p opts-bevy --example texture_scan -- <dir> --generate` prints the DDS-chain and mip-cost numbers for a texture folder.

## Open problems and questions

- Night/dusk: Vivid is 8-12 % darker than Bevy default at 19:00-21:00 (tonemapper and saturation, not ambient). Options for the project lead: accept, or let Vivid keep the Bevy default tonemapper below some sun elevation (a visible switch at dusk), or raise exposure at night. Headlights were not compared (off at session start); check with the lights on.
- *Natural* equals *Bevy default* until M-0060 step 2.
- Foliage/cutouts: the provisional heuristic `looks_like_cutout` (loader does not know the material's alpha mode when it creates a texture) instead of the real cutoff; M-0059 replaces it. Textures that are alpha-blended AND look like cutouts get a coverage-adjusted alpha (harmless).
- Block-compressed (DXT) textures without a chain (46,543) are untouched: they still alias at a distance (anisotropic filtering still applies). M-0058 phase C.
- Alpha-to-coverage is not done (M-0059); cutout edges are still hard but no longer pixelated.
- Moving shimmer, VRAM use of the extra levels (+33 % per uncompressed texture) and the weakest-GPU case were not measured (Q-TQ04).
- The ground shader keeps its own fixed anisotropy 8 sampler; it does not follow the setting.
- Final gates: fmt ok, clippy `-D warnings` ok, link check 0 broken, roadmap copy up to date; `cargo test --workspace`: 667 passed, 7 failed = exactly the baseline failures (cli default window size, input gating audit for rail.rs/vehicle_menu.rs, 5 loading-screen tests); rustdoc: exactly the baseline link errors. No new failure.
- Existing baseline failures of the gates (see above) are untouched.
- The plain `--screenshot` flag is not deterministic (ego camera sometimes starts in the cockpit): found while measuring, worth its own issue; `scripts/visual-ab.sh` avoids it.
- `vehicle.get` returned tile-local coordinates although the protocol documents absolute ones: fixed here.

