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
