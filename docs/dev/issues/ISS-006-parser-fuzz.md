# ISS-006: No fuzz or property tests for untrusted OMSI parsers

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-006 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (robustness) |
| **Milestone** | [M-0018](../../src/roadmap/phase-3/M-0018.md) 3D Mesh Asset Loaders and Conversion Bridge (`.o3d` and `.x`); the `.bus`, `.sli` and `.hof` readers are owned by their own milestones and are covered here only for truncation |
| **Feature audit** | none (project audit PRJ-04, architecture 2) |
| **Status** | done 2026-11-22 for truncation tests; fuzz targets open |

## Problem

There is no `fuzz/` directory and no proptest, quickcheck or arbitrary dependency under `crates/`. Pseudo-random tests exist for `.x`, `.o3d`, model and operational. There are no truncation tests for `.bus`, `.sli` or `.hof`.

## Work

1. Truncation tests in code for `.bus` (`BusManifest::parse_bytes`), `.sli` (`SplineProfile::parse`) and `.hof` (`HofFile::parse_bytes`): every prefix of a small in-code sample must return `Ok` or `Err` and must not panic. No copied game files.
2. Check whether the repository has a fuzz setup. It has none, and cargo-fuzz is not installed; no toolchain is installed for this issue. The cargo-fuzz targets for `.x`, `.o3d`, `.bus` and the map formats stay open.
3. Removal check for each truncation test, as in ISS-005.

## Outcome

- Truncation tests in `crates/opts-omsi/tests/truncation_parsers.rs` (4 of 4 pass, `txb-trunc-final.log`): every prefix of an in-code sample of text `.x` (`parse_x`), `.bus` (`BusManifest::parse_bytes`), `.sli` (`SplineProfile::parse`) and `.hof` (`HofFile::parse_bytes`) returns without panic; each full sample parses. No game file is read.
- Removal check for the truncation tests was not run; the tests were run only in their passing form. The earlier X-format panics in `txb-x.log` do not reproduce on the current code.
- Still open: the cargo-fuzz targets for `.x`, `.o3d`, `.bus` and the map formats. No `fuzz/` directory or toolchain exists; cargo-fuzz is not installed.
