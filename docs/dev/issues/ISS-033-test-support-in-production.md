# ISS-033: Test support in the production library and dead-code allows

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-033 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (architecture, test hygiene) |
| **Milestone** | [M-0040](../../src/roadmap/phase-5/M-0040.md) Code-quality debt (test support feature-gated 2026-10-11, commit 28249ea8); the structure ratchet is [M-0147](../../src/roadmap/phase-15/README.md) |
| **Audit** | Project audit 2026-11-18, architecture 6 (PRJ-13) |
| **Status** | in progress 2026-11-22 on `agent/test-fixtures-2`. The gate is on master (`28249ea8`), so the 2026-11-18 evidence is stale for the module. Done: production build `cargo build -p opts-game` without the feature finished (`Finished dev profile in 3m 56s`), `cargo test -p opts-game --lib` 757 passed, 0 failed. Decision: both dead-code allows stay, each with a one-line justification in code (`ServerGuard` in `opts-bevy/src/control/mod.rs`; `Binding` in `opts-game/src/help/controls.rs`, already there). Open: the gate-removal check (step 3). |

## Problem

The 2026-11-18 audit found two things in the production code.

1. `pub mod testing;` in `crates/opts-game/src/catalogue_service.rs` (line 144) builds a miniature OMSI installation for the integration tests. The audit saw it without a `cfg`. On master it is gated by `#[cfg(any(test, feature = "test-support"))]`, and `crates/opts-game/Cargo.toml` declares `test-support = []`. The six integration targets that use it (`wizard_tests`, `golden_screens`, `ui_rules`, `a11y_matrix`, `hud_matrix`, `loading_tests`) carry `required-features = ["test-support"]`.
2. Two non-test `dead_code` allows:
   - `crates/opts-bevy/src/control/mod.rs` (line 56-57): `#[allow(dead_code)] struct ServerGuard(ServerHandle)`. The field is never read; it keeps the control server alive for the app's lifetime.
   - `crates/opts-game/src/help/controls.rs` (line 15-16): `#[cfg_attr(not(test), allow(dead_code))]` on `Binding`. The fields `keys` and `source` are read only by the tests in that file.

## Work

1. Confirm the gate: a production build without `test-support` has no `testing` module; `cargo test -p opts-game` passes.
2. Try to remove each allow. Justify in a one-line comment what cannot be removed.
3. Removal check: drop the `cfg` gate for one build and show that the production build then compiles the module, then restore with `git checkout`.
4. Update this page, the register row and the board line in the same commit as the code.

## Done when

- The production build (`cargo-lowmem.sh build -p opts-game`) succeeds without the feature, and `cargo-lowmem.sh test -p opts-game` passes.
- Each dead-code allow is either removed, or has a one-line comment that says why it stays.
- The removal check shows the production build change when the gate is removed, and the tree is restored.
- Board line and register row set to done with the commit hash.
