# 🛠️ Phase 2 Kickoff — `.o3d` Parser Implementation

You are implementing the first parser of Phase 2 for the OPTS project:
`opts_core::formats::o3d`. The specification is 🟢 Specified and has
been validated by mutation testing. Your job is to produce a parser
that reads `.o3d` files and produces a normalized `O3d` structure
suitable for use by `opts-bevy`.

This is an IMPLEMENTATION task. You are not re-analyzing the format.
You are not updating the spec unless you discover a fact the spec
misses. You are not writing rendering code.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crate
`crates/opts-core`

### Target module
`crates/opts-core/src/formats/o3d/`

### Spec
`docs/src/formats/binary/o3d.md` (🟢 Specified)
`docs/src/formats/binary/o3d-chunks.md` (chunk reference)
`docs/src/architecture/coordinates.md` (O3D → Bevy frame)

### Coding standards
`docs/src/contributing/standards.md` — apply these rules verbatim.

### Mutation fixtures
The first mutation kit generated test files. Confirm with the user
where they are located before running tests. Suggested path:
`/home/louisgoodnews/Dokumente/opts-mutation-kit/fixtures/`

Do not commit any OMSI asset. The kit fixtures are self-authored and
may be committed only if the user approves.

---

## 2. Legal and Safety Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not commit any `.o3d` file from an OMSI installation.
- Do not copy code from `O3DView`, `O3DParse`, `Blender-O3D-IO-Public`,
  or any other community tool. Apply "read, close, write" if you
  consult them for concepts — but prefer not to consult them at all.
- Do not modify the spec unless you discover a fact it does not
  document (see Section 7).
- Do not add dependencies that are not already in the workspace
  `Cargo.toml` without user approval.
- Do not write `unsafe` code.
- Do not panic in library code paths. Return `Result`.

### ✅ Permitted

- Read the spec, the coding standards, and the mutation fixtures.
- Write Rust code in `opts-core`.
- Write unit tests in `#[cfg(test)] mod tests`.
- Write integration tests in `crates/opts-core/tests/`.
- Write rustdoc and module docs.
- Write `docs/crates/opts-core.md` if it does not exist.
- Update the spec's Open Questions table if you discover an
  undocumented fact (see Section 7).

---

## 3. Preconditions

Verify and report before writing code:

1. `docs/src/formats/binary/o3d.md` has status 🟢 Specified.
2. `docs/src/architecture/coordinates.md` exists.
3. `docs/src/contributing/standards.md` exists.
4. `crates/opts-core/` exists with a `Cargo.toml`.
5. `crates/opts-core/src/lib.rs` exists.
6. The encoding-sweep pass has completed (no mojibake in docs).
7. P-001 audit has returned clean since the encoding sweep.

If any precondition fails, STOP and report. Do not attempt to
work around missing preconditions.

---

## 4. Scope

### In Scope

- The `opts_core::formats::o3d` module and its submodules.
- Parsing the `.o3d` binary layout per the spec.
- Decoding the vertex buffer and index buffer.
- Decoding the material table.
- Decoding the 0x79 transform matrix as data (not baked in).
- Preserving unknown chunks for future analysis.
- Error type `O3dParseError` using `thiserror`.
- Unit tests for each parser stage.
- Integration tests against the mutation fixtures.
- Rustdoc for every public item.
- Module-level `//!` docs.
- `docs/crates/opts-core.md` crate documentation.

### Out of Scope

- Rendering. That belongs in `opts-bevy`.
- Serialization of the parsed structure back to `.o3d`. That is a
  future task (round-trip fidelity).
- Skinning or skeletal animation. If the `.o3d` files in the
  fixtures contain skinning data, parse it into a raw chunk and
  defer interpretation.
- `.x` parsing. That is a separate Phase 2 task.
- Any file I/O inside `opts-core`. Parsers are pure; the caller
  provides bytes.

---

## 5. Methodology

Follow these stages in order. Report after each one.

### Stage 1 — Crate Structure

1. Confirm `crates/opts-core/Cargo.toml` has:
   - `binrw` in dependencies
   - `thiserror` in dependencies
   - `glam` in dependencies
   - `tracing` in dependencies
   If any is missing, add it via the workspace dependency table,
   not as a direct version spec.

2. Create the module tree:
   ```
   crates/opts-core/src/formats/
   ├── mod.rs
   └── o3d/
       ├── mod.rs
       ├── error.rs
       ├── header.rs
       ├── chunks.rs
       ├── vertex.rs
       ├── index.rs
       ├── material.rs
       └── parse.rs
   ```

3. Add `pub mod o3d;` and `pub mod formats;` to the appropriate
   parent modules.

4. Verify `cargo check -p opts-core` succeeds on the empty module
   tree.

### Stage 2 — Error Type

1. In `error.rs`, define:
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum O3dParseError {
       #[error("file truncated: expected at least {expected} bytes, got {actual}")]
       Truncated { expected: usize, actual: usize },
       #[error("unknown chunk id: 0x{0:08X} at offset {1}")]
       UnknownChunk(u32, usize),
       #[error("invalid UTF-8 in string table at offset {0}")]
       InvalidUtf8(usize),
       #[error("unsupported o3d version: {0}")]
       UnsupportedVersion(u32),
       #[error("vertex format mismatch: expected {expected} bytes per vertex, got {actual}")]
       VertexFormatMismatch { expected: usize, actual: usize },
       #[error("index out of bounds: {index} >= {count}")]
       IndexOutOfBounds { index: u32, count: usize },
   }
   ```

2. Do NOT add `Io(#[from] std::io::Error)`. Parsers do not do I/O.

3. Add a `type Result<T> = std::result::Result<T, O3dParseError>;`
   alias.

### Stage 3 — Header

1. In `header.rs`, define the header struct per the spec.
2. Implement `Header::parse(reader: &mut Cursor<&[u8]>) -> Result<Header>`.
3. Use `binrw` derive where possible.
4. Validate:
   - Version is one of the supported set (1, 3, 4, 5, 7).
   - File size field matches the actual input length, if the spec
     says the header contains a size field.
5. Reject versions not in the supported set with
   `O3dParseError::UnsupportedVersion`.

### Stage 4 — Chunks

1. In `chunks.rs`, define the chunk iterator or dispatcher.
2. Read chunks sequentially. For each chunk:
   - Read chunk ID (u32).
   - Read chunk size (u32).
   - Dispatch to a per-chunk parser if known.
   - Otherwise, record the raw bytes in `unknown_chunks`.
3. Implement handlers for the known chunks per
   `docs/src/formats/binary/o3d-chunks.md`.
4. The 0x79 matrix chunk is stored as data. Do NOT apply it.

### Stage 5 — Vertex Format

1. In `vertex.rs`, decode the vertex buffer per the spec.
2. Support the vertex formats documented. If the spec lists a
   format you do not have a fixture for, implement it but mark
   the code path with `// TODO: verify with fixture` and an
   integration test that ignores.
3. Produce a `Vec<Vertex>` with position, normal, UV, and color
   where present. Use `glam::Vec3`, `glam::Vec2`, `[f32; 4]`.

### Stage 6 — Index Format

1. In `index.rs`, decode the index buffer.
2. Support both 16-bit and 32-bit indices (the header flags tell
   you which).
3. Produce a `Vec<[u32; 3]>` of triangles.

### Stage 7 — Material Table

1. In `material.rs`, decode the material table per the spec.
2. Each material has:
   - Diffuse RGBA (4 floats)
   - Specular RGB (3 floats)
   - Emissive RGB (3 floats)
   - Specular power (1 float)
   - Texture references
3. Store as `Material { diffuse: [f32; 4], specular: [f32; 3],
   emissive: [f32; 3], power: f32, textures: Vec<String> }`.
4. Do not interpret the alpha channel. Store it as-is.

### Stage 8 — Normalized Struct

1. In `mod.rs`, define the public `O3d` struct:
   ```rust
   pub struct O3d {
       pub version: u32,
       pub header_flags: u32,
       pub vertices: Vec<Vertex>,
       pub indices: Vec<[u32; 3]>,
       pub materials: Vec<Material>,
       pub transform: Option<[[f32; 4]; 4]>,
       pub unknown_chunks: Vec<UnknownChunk>,
   }
   ```
2. Implement a `pub fn parse(bytes: &[u8]) -> Result<O3d>` function
   in `parse.rs`.
3. The function is the single entry point. It delegates to the
   submodules in order.

### Stage 9 — Unit Tests

For each submodule, write unit tests in a `#[cfg(test)] mod tests`
block. Required cases:

- **Header:** valid header, truncated header, unsupported version.
- **Chunks:** valid chunk sequence, unknown chunk preserved,
  truncated chunk.
- **Vertex:** valid vertex buffer, buffer shorter than declared
  count, misaligned float.
- **Index:** valid 16-bit indices, valid 32-bit indices, index
  out of bounds.
- **Material:** valid table, table shorter than declared count.

Use small hand-authored byte arrays, not file I/O. Each test should
fit in under 40 lines.

### Stage 10 — Integration Tests

1. Create `crates/opts-core/tests/o3d_parse.rs`.
2. For each fixture from the mutation kit:
   - Load the file via `std::fs::read`.
   - Parse it.
   - Assert the vertex count, index count, and material count match
     what the fixture was designed to test.
3. Fixtures to use (per the mutation report):
   - `t01_axes.o3d` — verify vertex coordinates are stored as
     declared
   - `t01_winding_rev.o3d` — verify triangle winding is preserved
   - `t02_uv_orientation.o3d` — verify UV values are correct
   - `t03_mat_group_*.o3d` — verify material floats land in the
     right groups
   - `t04_matrix_*.o3d` — verify the matrix chunk is preserved as
     data
   - `t05_v*.o3d` — verify all supported versions parse
   - `t05_f*.o3d` — verify flags are read correctly
   - `t06_multimat.o3d` — verify per-face material indices
4. If a fixture is missing, note it and skip that test. Do not
   fabricate a replacement.

### Stage 11 — Robustness Tests

1. Fuzz the parser with:
   - Empty byte slice
   - Single-byte input
   - 512 random bytes (fixed seed)
   - Truncated valid file (first N bytes for N in 1..file_len)
2. The parser must return `Err` for all of these, never panic.
3. Use the `proptest` crate if it is available; otherwise write
   a deterministic loop.

### Stage 12 — Documentation

1. Every module gets a `//!` header explaining:
   - What this module parses
   - Which section of `o3d.md` it corresponds to
2. Every public item gets `///` with:
   - Description
   - Arguments
   - Return value
   - Errors
3. Crate-level docs in `crates/opts-core/src/lib.rs`:
   - What the crate does
   - How it's organized
   - A worked example of parsing an `.o3d`
4. `docs/crates/opts-core.md`:
   - Crate purpose
   - Public API surface (high-level)
   - Dependencies and why
   - Design decisions
   - Cross-references to specs

### Stage 13 — Verify

Run:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p opts-core --all-features
```

Fix every warning. Do not use `#[allow(...)]` to silence them.
If a warning is genuinely inapplicable, add a comment explaining
why and use a targeted allow.

---

## 6. Deliverables

At the end of the task, the following should exist:

- ✅ `crates/opts-core/src/formats/o3d/` module tree with all files.
- ✅ `O3d` struct with public fields.
- ✅ `parse(bytes: &[u8]) -> Result<O3d>` public function.
- ✅ Unit tests for each submodule.
- ✅ Integration tests against mutation fixtures.
- ✅ Robustness tests for malformed input.
- ✅ Rustdoc for every public item.
- ✅ Module docs for every module.
- ✅ `docs/crates/opts-core.md`.
- ✅ All tests pass, clippy clean, fmt applied.

---

## 7. Spec Updates

If during implementation you discover:

- A field whose meaning the spec does not document.
- A chunk ID not listed in `o3d-chunks.md`.
- A behavior that contradicts the spec.

Then:

1. **STOP implementing.**
2. Document the discovery in the spec's Open Questions table with
   a new Q-NNN ID.
3. Report the discovery to the user.
4. Resume only after the user confirms the spec update.

Do NOT guess at meanings. Do NOT silently handle undocumented
fields. A spec update is cheaper than a wrong parser.

---

## 8. Reporting Cadence

After each stage, pause and report:

```
## Phase 2 Progress — Stage N

**Stage:** <name>
**Files created:** <list>
**Files modified:** <list>
**Tests added:** <count>
**Test result:** <pass/fail>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

Do not proceed without confirmation. Implementation can drift from
intent; the checkpoints catch drift early.

---

## 9. Exit Criteria

- ✅ All Stage 1–13 stages completed.
- ✅ All deliverables from Section 6 exist.
- ✅ All tests pass.
- ✅ `cargo clippy -- -D warnings` passes.
- ✅ `cargo fmt --check` passes.
- ✅ No `unwrap()` in library code paths.
- ✅ No `unsafe`.
- ✅ No `println!` in library code.
- ✅ Every new file is under 500 lines.
- ✅ Every new function is under 80 lines.
- ✅ The `.o3d` spec has no new undocumented Open Questions.
- ✅ `dev/BOARD.md` reflects the completed work item.
- ✅ A single commit per logical unit of work.

When finished, respond with a summary containing:

- Files created / modified
- Lines added / removed
- Test counts and results
- Any spec updates made
- Recommended follow-up items

Then respond with the exact string:
"O3D PARSER COMPLETE — ready for P-006"