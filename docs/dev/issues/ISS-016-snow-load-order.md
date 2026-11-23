# ISS-016: Snow cover on scenery depends on texture load order

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-016 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0121](../../src/roadmap/phase-12/M-0121.md) Wetness and snow on vehicles and scenery |
| **Feature audit** | FEAT-09 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | done (agent/snow-refresh, 2026-11-22, from master b8a4b706; on-screen evidence open on M-0121) |

## Problem

The snow flag is set each frame from the session weather, but scenery textures read it only when they load. A texture already loaded keeps the variant it was loaded with, so scenery shows snow only when its texture loaded after the weather changed to snow.

Evidence (master b8a4b706):

- `crates/opts-bevy/src/season.rs` lines 22-29: `set_snow_cover` stores the flag; its doc says already loaded objects keep their textures.
- `crates/opts-bevy/src/season.rs` lines 42-49: `variant_folders` puts `WinterSnowfall` and `WinterSnow` ahead of the base folders when the flag is on.
- `crates/opts-bevy/src/model/texture_load.rs` line 67: `ContentLoader::texture` reads `snow_cover()` once per call; `cache.textures` is keyed by file path, so a loaded texture is never looked up again.
- `crates/opts-game/src/in_game/omsi_world/scenery_objects.rs` line 146: `sync_scenery_clock` sets the flag from `precip_type` each frame.
- `season.rs` test at line 68 checks folder order only; no test covers a change after load.
- `docs/src/architecture/weather.md` section 6 ("Seasons") states the limit: "objects loaded before a change keep their textures".

## Work

1. Refresh loaded scenery textures when the flag changes (approach chosen in the outcome section below).
2. Make the `graphics.snow_intensity` comment true or false (`crates/opts-bevy/src/weather/effects.rs`, the `snow_percent` field doc).
3. Test that changes the flag after load and asserts the variant changes; the test must fail when the refresh is removed.

## Outcome

Approach: each texture the loader reads is recorded with its file under both cover states (`model/season_refresh.rs`, `ContentLoader::texture`). Each frame, `refresh_season_textures` compares the cover flag with the state each recorded image shows; on a change it decodes the other file and swaps it into the same image asset (`Assets::insert`), so every material that uses the handle follows without a rebuild. A missing variant keeps the image; a removed asset is dropped.

Results (targeted, `cargo-lowmem.sh`, `CARGO_TARGET_DIR=opts-scratch/target-featuregap`, after `clean -p opts-bevy`):

- `season` tests, with the refresh: `test result: ok. 6 passed; 0 failed; 629 filtered out` (`work/snow/season-run5.log`).
- Removal check: the swap removed (the `images.insert` in `SeasonTextures::refresh`), `a_texture_loaded_before_the_cover_changes_shows_the_snow_file_after_the_change` FAILED with `assertion \`left == right\` failed` at `season_refresh.rs:188`, exit 101 (`work/snow/removal.log`).
- Restored: the file was copied back from `work/snow/season_refresh.rs.good` (`cmp` equal), and `season` passes again, 6 passed (`work/snow/restore-pass.log`).

Limit: derived copies are not refreshed. A texture made by `opaque_variant` (blended or alpha-stripped materials) or the composed transmap is a new image asset, not the file's decode, so it keeps the cover state it was made in until the object reloads. Vehicles and objects loaded before a change follow the swap only through the shared image asset.

On-screen (2026-11-23, agent/snow-onscreen, from master d3fd638e + 6395d448, binary built with `CARGO_TARGET_DIR=opts-scratch/target-featuregap`): one game run on `Bad_Huegelsdorf_2020`, vehicle `MAN_SD200/MAN_SD77.bus`, hour 12, `graphics.snow_intensity` 100, sim paused, camera at a fixed pose beside the player vehicle. Weather changed from `Clear Summer Day` (`#CAVOK`, snow false) to `Winter Snowfall` (`Starker Schneefall`, snow true) at the same time of day. Screenshots (outside the repo): `work/screens/snow-cover-before-2026-11-23.png` and `work/screens/snow-cover-after-2026-11-23.png`. Seen: the lawn and the shrub and grass objects in the foreground change from green to snow-covered, with snow in the air; the trees and parked cars do not visibly change. Limit: the pair does not separate the texture refresh from the ground snow tint that already existed, and no run with the refresh removed was made, so the change is not attributed to ISS-016 by this evidence alone. Game log: `work/snow-onscreen/game-2026-11-23.log` (no refresh-specific line). The first `sim.get` after the change timed out once (1005) and then answered.

Note on the test run: a shared target directory let another session's build overwrite the test binary. Runs before the `clean -p opts-bevy` used a stale binary without the refresh tests and are not used as evidence.

## Done when

- The removal-checked test passes and fails with the refresh removed.
- The spec (`weather.md`, section 6) and the code agree.
- The board line is done with the commit hash.
- On-screen evidence stays open on M-0121 (not checked in this issue).
