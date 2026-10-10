# 📜 P-005 — Implementation

| Metadata | Value |
| :--- | :--- |
| **ID** | P-005 |
| **Category** | core |
| **Prerequisites** | P-004 |
| **Follow-up** | P-006 |
| **Last Updated** | 2026-10-11 |

**When to use:** After P-004, when the user is ready to execute.

**Purpose:** Execute the approved implementation plan with full
documentation.

### Prompt

```
You are implementing an approved OPTS milestone.

## Inputs
- The approved milestone file from P-002
- The Adversarial Review from P-003
- dev/BOARD.md (updated in P-004)
- All specs referenced by the plan
- The OPTS workspace structure in crates/

## Task

Execute the implementation plan step by step. For each step:

### Before coding
1. Restate the step in one sentence.
2. Confirm its precondition is met.
3. Identify the file(s) to create or modify.

### While coding
1. Write **module-level documentation** at the top of every new
   module using `//!` doc comments.
2. Write **rustdoc** (`///`) for every public item:
   - Functions: description, arguments, return value, errors, panics
   - Structs and enums: purpose and semantics of each field/variant
   - Traits: contract and invariants
3. Write **inline comments** for non-obvious logic only. Do not
   comment trivial code.
4. Follow the crate boundary rules:
   - `opts-core` has no Bevy dependency
   - `opts-sim` has no Bevy dependency
   - `opts-bevy` is the only crate that touches Bevy
5. Use `thiserror` for library errors and `anyhow` for binary errors.
6. Emit diagnostics via `tracing`, never `println!`.

### After each step
1. Run `cargo check`, `cargo clippy`, `cargo fmt`, and `cargo test`
   for the touched crates.
2. Fix any failures before proceeding.
3. Update the corresponding work item in dev/BOARD.md to "Review"
   or "Done" as appropriate.

### Documentation in docs/crates/
For every crate you modify, write or update a document in
`docs/crates/<crate-name>.md` covering:
- The crate's purpose
- Its public API surface (high-level)
- Its dependencies and why they're needed
- Notable design decisions
- Cross-references to relevant specs

This is **module-level documentation**. It complements rustdoc — the
rustdoc lives in the source, the crate doc lives in the mdBook.

### Commits
Commit one logical unit of work per commit. Commit messages:
`<type>(<scope>): <short description> (M-NNNN)`

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`.
Scope: crate name (e.g., `opts-core`).

## Constraints
- ❌ Do not touch OMSI binaries.
- ❌ Do not copy GPL code.
- ❌ Do not commit assets.
- ❌ Do not modify the milestone's scope.
- ✅ If a step fails or reveals a spec gap, **stop** and report.
- ✅ If a spec's Open Question becomes answerable during
  implementation, update the spec's "Resolved Questions" section.

## Output
After each step, report:
- Step number and title
- Files created / modified
- Commands run and their result
- Work items moved on the BOARD
- Blockers, if any

At the end of the milestone, produce a "Milestone Summary" with:
- Acceptance criteria check (pass/fail per criterion)
- LOC added / removed
- New public API surface
- Any spec updates made
- Recommended follow-up items

When finished, respond with: "IMPLEMENTATION COMPLETE — ready for P-006"
```

### Exit Criteria

- ✅ All implementation plan steps completed.
- ✅ All acceptance criteria pass.
- ✅ Every new module has `//!` docs.
- ✅ Every public item has `///` docs.
- ✅ `docs/crates/<crate>.md` written or updated.
- ✅ Work items moved to Done on the BOARD.
- ✅ Agent responds with the ready string.

---

## Related Prompts

- [Workflow](../workflow.md)
- [Prompt index](../README.md)
