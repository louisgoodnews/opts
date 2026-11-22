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

## Outcome (2026-11-22, `agent/settings-test-hygiene`)

- Docs: the HDR dependency is in the `graphics.bloom` row of `docs/src/reference/settings.md`, in `set-bloom-desc` (en and de), and in `lighting-system.md` section 9.1. The reference page was checked by the generator test and matches it (`reference_page_is_up_to_date ... ok`); no regeneration was needed beyond the row edit.
- Tests: `cargo test -p opts-bevy --lib bloom` gives `test result: ok. 7 passed; 0 failed` (includes `bloom::tests::bloom_needs_hdr_and_does_not_switch_it_on` and the new `bloom::tests::bloom_is_removed_when_the_camera_loses_hdr`).
- Removal check: in `crates/opts-bevy/src/bloom.rs` the removal branch of `sync_bloom` was changed to `if have.is_some() && want.is_none()`, so Bloom is kept on an HDR-less camera. The new test then fails (`bloom_is_removed_when_the_camera_loses_hdr ... FAILED`, panic at `bloom/tests.rs:77`, 6 passed, 1 failed). `bloom.rs` was restored from git and matches HEAD exactly. `bloom_needs_hdr_and_does_not_switch_it_on` still passed under the mutation, so the new test is the one that guards removal.
- Limits: the Low (0.03) and High (0.10) intensities are still unmeasured in the game; the lighting document marks them so. The game was not run for this issue.
