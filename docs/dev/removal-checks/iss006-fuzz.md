# ISS-006 fuzz-style tests: removal check

Date: 2026-11-24. Branch `agent/iss006-fuzz` (base `master` at `3a482e9d`). Test:
`crates/opts-omsi/tests/fuzz_parsers.rs` (helpers in `crates/opts-omsi/tests/fuzz_parsers/`: allocator, mutations, samples, reader table). Re-run on that final layout. Readers: `.x` (`parse_x`), `.bus`
(`BusManifest::parse_bytes`), `.sli` (`SplineProfile::parse`), `.hof` (`HofFile::parse_bytes`).

## What is removed

The ISS-005 bound in `TokenReader::parse_count` (`crates/opts-omsi/src/formats/x/parse/reader.rs`):
the `if count > left` check was changed to `if false && count > left`. That is the only edit
in the check. Every list count in `parse/mesh.rs` and `parse/material.rs` goes through this call.

## Result

| Step | Result |
| :--- | :--- |
| Baseline, bound in place | `fuzz_parsers`: 4 passed, 0.03 s |
| Bound removed | `x_survives_prefixes_and_mutations_within_the_count_bound` FAILED; the other three readers passed |
| Failure message | `x: digit run 2 replaced by "4000000000": a single allocation of 48000000000 bytes exceeds the 35072-byte bound for a 484-byte input` |
| Restored | `cmp` clean against a copy taken before the edit, and against `master`; `sha256sum` `211d895e…aeea00` before and after |
| Baseline after restore | `cargo test -p opts-omsi --no-fail-fast`: all green (exit 0) |

The failing input is the sample with its first vertex-count token (`4` → `4000000000`). The
48 GB request is `4000000000 × 12` bytes of `Vec<Vec3>` storage, which `Vec::with_capacity`
requests before the first element is read.

## How the allocation bound works

The test binary installs a counting `GlobalAlloc` that forwards to `System` and records the
largest single request. For each input, the bound is `64 × input length + 4096` bytes. A
count-driven allocation that stays within the input is far under this bound; a count taken
from the file's declared value is not.

## Limits of this check

- Only the `.x` bound was removed. The other three readers have no count-driven allocation
  (`hof` counts drive loops that stop at the end of the lines), so their tests pass with or
  without the bound; they guard against panics and allocation blow-up only.
- The removal shows that the bound is observable through the allocation counter. It does not
  show that the other ISS-005 count tests (`count_tests` in `src/formats/x/count_tests.rs`) fail
  without the bound: those are unit tests, not run here.

# Extension: map tile, map terrain and o3d readers (branch `agent/iss006-port`)

Date: 2026-11-24. Readers now in `crates/opts-omsi/tests/fuzz_parsers.rs`: `.x`, `.bus`, `.sli`,
`.hof` (text), map tile (`MapTile::parse`, text), and the binary readers `.o3d` (`o3d::parse`) and
map terrain (`MapTerrain::parse`). Test count: 7 (was 4). Binary samples are built in code
(`samples::o3d`, `samples::terrain`); binary readers take every `u32`/`u16` window replaced by an
edge word (`mutate::windows`), text readers keep the digit-run replacement.

## Finding: o3d material pre-allocation (allocation bound, not a panic)

Before the first extension run, the `o3d` window test failed on the master code:
`parse_material_chunk` called `Vec::with_capacity(material_count)` from the file's `u16` count
before reading any record. Input: a 245-byte `.o3d` (version 3, one material) with its material
count replaced by `0xFFFF`. Requested allocation: 4,718,520 bytes, against a bound of 19,776 bytes
(`64 × len + 4096`). Not a panic, and capped by the `u16` count, but it is the ISS-005 rule the
test enforces. Fix (capacity only, no change to parsed results): capacity capped by the bytes
that remain, `material_count.min(remaining / MATERIAL_FIXED_BYTES)`.

## Removal check: o3d vertex count bound

| Step | Result |
| :--- | :--- |
| Target | `crates/opts-omsi/src/formats/o3d/vertex.rs` line 79: `if remaining_bytes < required_bytes {` |
| Edit | `if false && remaining_bytes < required_bytes {` (only edit) |
| Removed run | `o3d_survives_prefixes_windows_and_mutations_within_the_count_bound` FAILED; the other six passed (`removed.log`) |
| Failure | `o3d: panicked on prefix of length 9` at `vertex.rs:86` (slice past the end of the buffer for a declared 3-vertex count with no vertex data) |
| Restored | `cmp` clean against the saved copy and against `master`; `sha256sum` `fe07b7a4…4aa4` before and after |
| Gate after restore | `cargo test -p opts-omsi --no-fail-fast`: exit 0, all targets ok (`gate.log`); rerun after rustfmt: exit 0 (`gate2.log`) |

The bound is what keeps a declared count from indexing past the input. Without it the test
catches a panic rather than an allocation, since the slice is taken before `Vec::with_capacity`.
The material finding above is the case the removal check did not cover; it was caught by the
window test before the fix and is shown by the failing first run, not by a removal edit.

## Port to the split crates (branch `agent/iss006-port`, from master `95a8a588`)

Date: 2026-11-24. The o3d reader now lives in `crates/opts-omsi-binary/src/formats/o3d/material.rs`
(the crate split, docs/dev/crate-split-omsi.md). The fuzz tests stay in `crates/opts-omsi/tests/`.
The material fix is ported to that path. Note: the rows above that name `crates/opts-omsi/src/formats/o3d/`
refer to the pre-split location; that code is now `crates/opts-omsi-binary/src/formats/o3d/`.

To keep `opts-omsi-binary` within its budget, the material fix was paid for by inlining the 4-byte
`read_slice_4` helper in `material.rs` as `fixed_buf[N..N + 4].try_into().unwrap()` at its eleven call sites
(same bytes read, same result). Measured `opts-omsi-binary` is 3135 code lines, and its budget entry is lowered
from 3136 to 3135 in `docs/dev/crate-budget.txt`. No budget was raised.

| Step | Result |
| :--- | :--- |
| Target | `crates/opts-omsi-binary/src/formats/o3d/material.rs`, `Vec::with_capacity(material_count.min(remaining / MATERIAL_FIXED_BYTES))` |
| Edit | cap removed: `Vec::with_capacity(material_count)` (only edit) |
| Removed run | `o3d_survives_prefixes_windows_and_mutations_within_the_count_bound` FAILED; the other six fuzz tests passed (`removed.log`) |
| Failure | `o3d: window replacement 1375: a single allocation of 4718520 bytes exceeds the 19776-byte bound for a 245-byte input` |
| Restored | `cmp` clean against the saved fixed copy (`material.fixed.rs`); `sha256sum` `931747de…c67` after restore, matching the fixed copy |
| Fixed run | `fuzz_parsers`: 7 passed, 0 failed (0.04 s) |
