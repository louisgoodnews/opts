# ISS-016: Snow cover on scenery depends on texture load order

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-016 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (correctness) |
| **Milestone** | [M-0121](../../src/roadmap/phase-12/M-0121.md) Wetness and snow on vehicles and scenery |
| **Feature audit** | FEAT-09 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | in progress (agent/snow-refresh, started 2026-11-22, from master b8a4b706) |

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

Pending. This section is filled in at completion.

## Done when

- The removal-checked test passes and fails with the refresh removed.
- The spec (`weather.md`, section 6) and the code agree.
- The board line is done with the commit hash.
- On-screen evidence stays open on M-0121 (not checked in this issue).
