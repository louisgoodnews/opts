# ISS-018: Bloom needs HDR (undocumented); intensities unmeasured; one test

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-018 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (correctness, docs) |
| **Milestone** | [M-0122](../../src/roadmap/phase-12/M-0122.md) Graphics overhaul (bloom); the setting is in the [settings registry](../../src/roadmap/phase-9/M-0063.md) (M-0063) |
| **Feature audit** | FEAT-11 in [feature-audit-2026-11-18](../../audit/feature-audit-2026-11-18.md) |
| **Status** | in progress 2026-11-22 on `agent/settings-test-hygiene` |

## Problem

- `crates/opts-bevy/src/bloom.rs` removes `Bloom` from the main camera when the camera has no `Hdr`, so `graphics.bloom` does nothing while `graphics.hdr` is off. Neither the settings reference (`docs/src/reference/settings.md`, row `graphics.bloom`) nor the registry help text (`set-bloom-desc`) says so.
- The intensities of Low (0.03) and High (0.10) (`intensity_of`, Natural 0.06) have not been measured in the game. The lighting document's cost row covers only `NATURAL` (+0.3 ms).
- One test fn exists for the HDR dependency (`bloom_needs_hdr_and_does_not_switch_it_on`).

## Work

1. Add the HDR dependency to the bloom row of `settings.md` and to `set-bloom-desc` in `messages_lighting_en.ftl` and `messages_lighting_de.ftl` (German text in the de file).
2. Describe the dependency in `docs/src/architecture/lighting-system.md` where bloom is described (section 9.1), and mark the Low and High intensities as unmeasured.
3. Add a second unit test in `crates/opts-bevy/src/bloom/tests.rs`: bloom added while the camera is HDR is removed again when `Hdr` is taken off.

## Done when

- The settings reference is regenerated (`UPDATE_SETTINGS_DOC=1`) and the registry test passes.
- Both bloom unit tests pass.
- The intensities remain marked unmeasured in the lighting document.
- The board line and this file's status are updated on completion.
