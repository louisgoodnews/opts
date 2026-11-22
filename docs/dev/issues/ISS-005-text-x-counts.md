# ISS-005: Text X parser allocates from file-supplied counts

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-005 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (robustness) |
| **Milestone** | [M-0018](../../src/roadmap/phase-3/M-0018.md) 3D Mesh Asset Loaders and Conversion Bridge (`.o3d` and `.x`), which owns the `.x` reader |
| **Feature audit** | none (project audit PRJ-04, safety 6) |
| **Status** | done 2026-11-22 (bound and tests; see outcome) |

## Problem

`crates/opts-omsi/src/formats/x/parse/mesh.rs` and `parse/material.rs` call `Vec::with_capacity` on counts read from the file (`n_vertices`, `n_faces`, each face's index count, `n_normals`, `n_face_normals`, `n_coords`, the material face list). A corrupt or hostile count such as `4000000000` asks for gigabytes before the first element is read, and the abort is not a parse error.

The binary reader bounds its counts: `MAX_LIST = 50_000_000` in `x/binary.rs:26`, checked at `:104`, returning an error. That constant is not tied to the file size, so a 50 M-entry count in a 100-byte file still allocates about 200 MB before it fails.

## What the spec says

`docs/src/formats/binary/x.md` (text body, line 59) says a reader "must rely on the **declared counts**" and that declared counts match the records that follow (line 112: 99.4 % of sampled `Mesh` blocks are count-consistent). It gives no numeric limit. The only limit a count can honestly be checked against is the input itself: every element of a text list occupies at least one token, so a declared count larger than the tokens left in the file cannot be satisfied.

## Work

1. Read every file-supplied count through one helper that rejects a count larger than the remaining tokens (an error, no allocation).
2. Use the helper for all counts in `parse/mesh.rs` and `parse/material.rs`, including per-face index counts.
3. Test: a file with a huge declared count (`4000000000`) returns an error; a valid file still parses (the existing `x` tests cover this).
4. Removal check: remove the bound once, run the huge-count test, confirm FAIL, restore, confirm PASS.

## Outcome

- Done 2026-11-22 on `agent/text-x-bounds`. Every file-supplied list count in `parse/mesh.rs` and `parse/material.rs` goes through `TokenReader::parse_count` (`parse/reader.rs`), which rejects a count larger than the tokens left with an error and allocates nothing.
- Count tests `count_tests` (4 of 4): a valid quad still parses; huge vertex, face and per-face index counts (`4000000000`) return an error whose message names the tokens left (`txb-count-final.log`).
- Removal check: the bound was stripped (`if false && count > left`) and `count_tests::a_huge_vertex_count_is_an_error_not_an_allocation` FAILED with `a count beyond the file must fail: expected the count bound error, got UnexpectedToken { offset: 137, expected: "float number", found: "CloseBrace" }` (`txb-removal2.log`). `reader.rs` was restored and verified byte-identical to the saved copy; the test passes again with the bound.
- Earlier removal run (before the tests asserted the message) passed with the bound removed, because the parse fell through to end-of-file; the assertion now checks the message, which is why the test discriminates.
- Note: the removal FAILED line is the cargo-test output for the huge-count test only; the other two count tests were not removal-checked.
