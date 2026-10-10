# 🎨 Coding Standards

| Metadata | Value |
| :--- | :--- |
| **Applies to** | All Rust code in the OPTS workspace |
| **Enforced by** | `cargo fmt`, `cargo clippy`, CI, code review |
| **Status** | 🟢 Active |
| **Last Updated** | 2026-10-04 |

---

## 📖 Philosophy

OPTS is a long-lived, community-driven project. The code will be read
far more often than it is written, and it will be read by people who
did not write it — often years later, often while debugging.

**Optimize for the reader, not the writer.**

Three principles inform every rule in this document:

1. **Clarity over cleverness.** A boring function that any contributor
   can understand beats an elegant one-liner that requires a comment
   to explain.
2. **Data over hierarchy.** Model the domain as data flowing through
   functions. Do not model it as a web of objects sending messages.
3. **Bounded complexity.** Every file, function, and module has a
   size limit. When it exceeds the limit, it is split — not
   rationalized.

This document is organized into three layers: universal principles
that apply to any language, Rust-specific rules, and OPTS-specific
design conventions. When the layers conflict, the more specific layer
wins.

---

## 🌍 Part 1 — Universal Principles

These rules apply to any language, any project, any era. They are
older than Rust and will outlive it.

### 1.1 Name Things Honestly

A name is a contract. If the name lies, the reader will be misled.

| ❌ Bad | ✅ Good | Why |
| :--- | :--- | :--- |
| `data` | `vehicle_spec` | "data" describes nothing. |
| `process()` | `parse_bus_file()` | Verb + object beats verb alone. |
| `handle_thing()` | `dispatch_door_event()` | "Handle" and "thing" are noise. |
| `get_user()` | `find_user_by_id()` | "Get" hides the failure mode. |
| `is_valid` | `has_valid_geometry` | Specificity prevents misuse. |

**Booleans read as predicates.** `is_visible`, `has_children`,
`can_transition`. Never `visible_flag` or `visible_bool`.

**Units belong in the name.** `speed_mps`, `length_mm`, `angle_deg`,
`mass_kg`. When the unit is documented but not encoded, bugs of 1000×
follow.

**Plurals mean collections.** A variable named `pathpoint` holds one;
`pathpoints` holds many. Never `pathpoint_list` — that is redundant.

### 1.2 Make Illegal States Unrepresentable

Every invariant the code assumes should be enforced by the type
system, not by runtime assertions and comments.

```rust
// ❌ Bad: the "exactly one of these is set" rule lives in a comment.
struct Connection {
    tcp: Option<TcpStream>,
    unix: Option<UnixStream>,
    // One of the above MUST be set. Please.
}

// ✅ Good: the type encodes the invariant.
enum Connection {
    Tcp(TcpStream),
    Unix(UnixStream),
}
```

The compiler is the only code reviewer that never gets tired.
Offload every invariant you can to it.

### 1.3 Fail Fast, Fail Loud, Fail Local

- **Fail fast.** Detect errors at the earliest possible point, not
  after passing corrupted state to three other functions.
- **Fail loud.** Silent recovery is worse than crashing. A `WARN` log
  beats a `DEBUG` log beats silently ignoring.
- **Fail local.** Handle errors where they occur, not three call
  frames up with a bare `unwrap()`.

The only exception is user-facing input. OPTS must not crash on
malformed OMSI assets — it must log and continue. See the `.bus`
spec's "count mismatch" resolution for the pattern.

### 1.4 Prefer Composition Over Inheritance

Inheritance is a mechanism for sharing code. Composition is a
mechanism for **composing behavior**. Composition wins almost always.

- Rust has no inheritance. This is not a limitation; it is a
  correct default.
- Use enums for "one of these" relationships.
- Use traits for "can do this" relationships.
- Use structs for "has these" relationships.
- Use functions for everything else.

### 1.5 Minimize Nesting

Every level of nesting is a small tax on the reader. Beyond three
levels, comprehension drops sharply. Beyond four, readers skip.

```rust
// ❌ Bad: four levels deep.
fn process(input: &str) -> Option<Output> {
    if !input.is_empty() {
        for line in input.lines() {
            if line.starts_with('[') {
                if let Some(kw) = parse_keyword(line) {
                    // ... the actual logic
                }
            }
        }
    }
    None
}

// ✅ Good: guard clauses flatten the flow.
fn process(input: &str) -> Option<Output> {
    if input.is_empty() {
        return None;
    }
    input.lines().filter_map(try_parse_keyword).next()
}
```

**Techniques to flatten:**
- **Guard clauses** — return early for edge cases.
- **`let else`** — bind or bail in one line.
- **Combinators** — `map`, `and_then`, `filter`, `flatten`.
- **Extract functions** — if a nested block does one thing, it is
  a function.

### 1.6 Keep Functions Small

A function should do one thing. If you can't state what it does in a
single sentence without "and", it should be two functions.

**Target:** ≤ 40 lines of code (excluding rustdoc, comments, and
blank lines).
**Hard limit:** 80 lines. Beyond that, refactor.

Small functions are easier to test, easier to name, easier to reuse,
and easier to delete. Deleting a small function costs nothing;
deleting a large one requires untangling.

### 1.7 Avoid Premature Abstraction

Three similar lines are not a pattern. A pattern is when the same
shape appears **four or more times** and has a stable interface.

> "Duplication is far cheaper than the wrong abstraction."
> — Sandi Metz

When you see the fourth duplication, extract. Before then, tolerate
the duplication. It is cheaper to copy-paste four times and then
abstract once than to abstract once and then spend three years
fighting the wrong interface.

### 1.8 Write Comments That Explain *Why*, Not *What*

The code already says *what*. A comment repeating the code is noise.

```rust
// ❌ Bad: restates the code.
// Increment i by 1.
i += 1;

// ✅ Good: explains a non-obvious decision.
// OMSI stores pathpoint indices per-segment, so index 0
// in the trailer is distinct from index 0 in the main
// segment. See `docs/src/formats/text/paths_cfg.md`.
let global_idx = (segment_idx, local_idx);
```

**Rule of thumb:** a comment should answer "why is this here?" or
"what surprised me while writing this?" It should never answer
"what does this line do?"

### 1.9 Test Behavior, Not Implementation

A test should fail when the behavior is wrong, and pass when the
behavior is right. It should not depend on internal details.

```rust
// ❌ Bad: tests the internal field name.
assert_eq!(parser.state, ParserState::Done);

// ✅ Good: tests observable behavior.
let result = parser.parse("fixture.bus").unwrap();
assert_eq!(result.manufacturer, "MAN");
```

Refactoring should never break tests. If it does, the tests were
testing the wrong thing.

### 1.10 Commit Messages Are Documentation

Every commit message is a permanent record. Write them for the
person reading `git log` six months from now.

**Format:** `<type>(<scope>): <imperative summary> (M-NNNN)`

- `<type>` — `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- `<scope>` — crate or module name (`opts-core`, `o3d`, `ci`)
- Summary — imperative mood, ≤ 72 characters, no trailing period
- `(M-NNNN)` — milestone reference, when applicable

Body paragraphs (separated by a blank line) explain **why**, not what.
The `diff` shows what changed; the body explains the reasoning.

---

## 🦀 Part 2 — Rust-Specific Practices

These rules apply specifically to Rust. They assume the reader is
already comfortable with ownership, borrowing, and traits.

### 2.1 Use `Result`, Never Panic, in Library Crates

**Rules for `opts-core` and `opts-sim`:**

- ❌ No `unwrap()`, `expect()`, `panic!()`, `unreachable!()`,
  `todo!()`, `unimplemented!()` in library code paths.
- ✅ Return `Result<T, E>` with a domain-specific error type.
- ✅ Use `thiserror` for library errors.
- ✅ Reserve `panic!` for genuine invariants: "this cannot happen
  unless the code is wrong."

**Rules for binary crates (`opts-game`, `opts-tools`):**

- ✅ Use `anyhow` for top-level error propagation.
- ✅ Use `.context("...")` to add human-readable context.
- ❌ Do not leak `anyhow::Error` into library APIs.

**Rules for tests:**

- ✅ `unwrap()` and `expect()` are fine. Panic is how tests fail.

### 2.2 Error Types

Use **`thiserror`** for structured errors. Do not use stringly-typed
errors, and do not use `Box<dyn Error>` in public APIs.

```rust
#[derive(Debug, thiserror::Error)]
pub enum O3dParseError {
    #[error("file too short: expected at least {expected} bytes, got {actual}")]
    Truncated { expected: usize, actual: usize },

    #[error("unknown chunk id: 0x{0:08X}")]
    UnknownChunk(u32),

    #[error("invalid UTF-8 in string table at offset {0}")]
    InvalidUtf8(usize),

    #[error("I/O error")]
    Io(#[from] std::io::Error),
}
```

The `#[from]` derive is the correct way to propagate underlying
errors. Do not manually convert when `?` and `#[from]` suffice.

**Never** implement `From<String>` for a library error type.
That is a path to stringly-typed errors.

### 2.3 Ownership and Borrowing

- **Borrow by default, own when necessary.** `&str` beats `String`;
  `&[T]` beats `Vec<T>` in function parameters.
- **Accept the most general type.** A function that only reads a
  string takes `&str`. A function that only reads a slice takes
  `&[T]`. Callers pass their type and convert with `.as_str()` /
  `.as_slice()` if needed — this is a zero-cost operation.
- **Return owned types from constructors.** A `parse_*` function
  returns `Result<Parsed, Error>`, not `Result<&Parsed, Error>`.
- **Clone deliberately.** A `.clone()` in a hot path is a bug. A
  `.clone()` in a cold path is fine, but should be a conscious
  choice, not a workaround for the borrow checker.

### 2.4 Prefer Iterators to Loops

Rust's iterator combinators are not a stylistic preference — they
compile to code that is at least as fast as an equivalent loop, and
they express intent more clearly.

```rust
// ❌ Imperative, noisy.
let mut visible = Vec::new();
for node in &scene.nodes {
    if node.visible && node.in_frustum(frustum) {
        visible.push(node.id);
    }
}

// ✅ Declarative, focused.
let visible: Vec<_> = scene.nodes
    .iter()
    .filter(|n| n.visible && n.in_frustum(frustum))
    .map(|n| n.id)
    .collect();
```

**When loops are fine:**

- Early termination is complex.
- Multiple outputs accumulate.
- Side effects dominate.
- `break` / `continue` with labels.

Do not contort a loop into an iterator chain to satisfy a rule.
Iterator chains are encouraged, not mandated.

### 2.5 Avoid `Clone` as a Borrow-Checker Workaround

If the compiler rejects your code and the fix is `.clone()`, that is
usually a signal that the ownership model is wrong.

Common patterns to prefer over clone:

| Instead of | Use |
| :--- | :--- |
| Cloning to pass to a function | Pass by reference |
| Cloning to satisfy lifetimes | Restructure lifetimes, or own the data at the top |
| Cloning in a loop | Hoist the clone out, or use `Rc`/`Arc` |
| Cloning to avoid borrow conflicts | Split the borrow, use indices, or `mem::take` |

`.clone()` in generated code from a derive macro is fine — this rule
is about hand-written clones.

### 2.6 Use `#[derive]` Aggressively

Derive is idiomatic and free of runtime cost. Derive these on all
data types unless there is a reason not to:

- `Debug` — always. Non-negotiable.
- `Clone` — when semantically meaningful.
- `PartialEq`, `Eq`, `Hash` — when the type is comparable.
- `Default` — when a sensible zero value exists.
- `serde::Serialize`, `serde::Deserialize` — for anything that might
  be persisted or loaded from data files.

**Never** implement `Debug` by hand unless the derived output leaks
secrets or overflows on recursive types. Manual `Debug` is almost
always a mistake.

### 2.7 Use `#[must_use]` for Functions with Side Effects

If a function's return value is important, mark it:

```rust
#[must_use = "ignoring the parsed result discards work"]
pub fn parse(input: &str) -> Result<Ast, ParseError> { ... }
```

The compiler will warn when callers ignore it. This prevents the
classic "I forgot to use the result" bug.

### 2.8 Prefer `&str` and `&[T]` to `&String` and `&Vec<T>`

This is the single most common beginner mistake. `&String` and
`&Vec<T>` force callers to have exactly that type; `&str` and `&[T]`
accept anything that derefs to them.

```rust
// ❌ Forces callers to own a String.
fn greet(name: &String) { ... }

// ✅ Accepts &String, &str, and string literals.
fn greet(name: &str) { ... }
```

### 2.9 `tracing`, Not `println!`

All diagnostics go through `tracing`. Never `println!` or `eprintln!`
in library code, and never in binary code except for actual user
output (CLI tool results).

```rust
use tracing::{debug, info, warn, error, trace};

trace!(offset, "reading vertex buffer");
debug!(chunk_id, size, "parsed chunk header");
info!("loaded {} pathpoints", pathpoints.len());
warn!(expected, actual, "count mismatch, proceeding anyway");
error!(?err, "failed to load file");
```

**Levels:**

| Level | Use for |
| :--- | :--- |
| `ERROR` | Something the user needs to act on. |
| `WARN` | Something wrong, but recoverable. |
| `INFO` | Major lifecycle events. |
| `DEBUG` | Development-time diagnostics. |
| `TRACE` | Very fine-grained tracing. |

Do not log expected behavior at `WARN`. A `WARN` means "you should
probably look at this".

### 2.10 Avoid `unsafe`

`unsafe` requires a documented justification. Before writing
`unsafe`, exhaust every alternative:

- Use `bytemuck` for byte-casting instead of raw pointers.
- Use `zerocopy` for zero-copy parsing.
- Use `bytes::Bytes` for reference-counted slices.

If `unsafe` is truly necessary, it must include:

1. A `// SAFETY:` comment explaining why the invariants are upheld.
2. A test that would fail if the invariants were violated.
3. A code review by a second contributor.

`unsafe` is not banned. It is gated.

### 2.11 Crate Boundaries

The workspace enforces a strict 4-layer unidirectional dependency direction per [ADR-0005](../adr/0005-crate-architecture.md):

```
opts-game (L3)
├── opts-bevy (L2)
│   ├── opts-core (L1)
│   └── opts-sim (L2)
│       └── opts-core (L1)
├── opts-gui (L2)
└── opts-logging (L0)

opts-tools (L3)
├── opts-core (L1)
└── opts-logging (L0)
```

**Rules:**

- `opts-logging` (Layer 0) **must not** depend on any OPTS internal crate.
- `opts-core` (Layer 1) **must not** depend on Bevy, and **must not** perform synchronous file I/O.
- `opts-sim` (Layer 2) **must not** depend on Bevy, and **must not** perform synchronous file I/O.
- Only `opts-bevy`, `opts-gui`, and `opts-game` are permitted to depend on Bevy.

Violating the boundary is a build error in code review. If a change
requires crossing the boundary, the boundary is wrong, and the ADR
record should be updated.

---

## 🏗️ Part 3 — OPTS Design Conventions

These rules are specific to how OPTS models its domain.

### 3.1 Data-Oriented, Not Object-Oriented

OPTS is a data-oriented project. This means:

- **Model the domain as data.** Prefer `struct` and `enum` to
  objects with behavior.
- **Put behavior in free functions.** Not methods on data types,
  unless the method is a natural operation on that type.
- **Prefer `fn process(input: Input) -> Output` to
  `input.process()`.**
- **Prefer `enum` + `match` to a class hierarchy.**
- **Prefer composition (fields) to inheritance (which Rust does
  not have).**

**Example:**

```rust
// ❌ Object-oriented style: data + method.
pub struct Parser {
    input: String,
    position: usize,
}

impl Parser {
    pub fn parse(&mut self) -> Result<Ast, ParseError> {
        // ...
    }
}

// ✅ Data-oriented style: pure functions over data.
pub struct ParserState {
    pub position: usize,
}

pub fn parse(input: &str) -> Result<Ast, ParseError> {
    let mut state = ParserState { position: 0 };
    parse_header(input, &mut state)
        .and_then(|_| parse_body(input, &mut state))
        .and_then(|_| parse_footer(input, &mut state))
}
```

**When methods are appropriate:**

- Natural operations on a single type: `Pathpoint::distance_to(&other)`.
- Trait implementations: `Display`, `Iterator`, `From`.
- Builders and constructors.
- Operations on external types where a free function would break
  the principle of least surprise.

### 3.2 ECS-Friendly Data Layout

Because OPTS targets Bevy, data structures should be designed to be
easy to break down into Entities, Components, and Systems.

- **Prefer plain data structs** — no hidden state, no interior
  mutability, no `Rc<RefCell<...>>`.
- **Avoid deep nesting.** A tree of structs becomes a tree of
  components in Bevy. Deep nesting means deep component hierarchies.
- **Prefer `Vec<Component>` to `ComponentContainer`** in parsed
  data. Flatten to collections early.

### 3.3 Immutability by Default

Structs should be immutable unless there is a reason to mutate.

```rust
// ❌ Mutable fields invite accidental state.
pub struct Vehicle {
    pub position: Vec3,
    pub velocity: Vec3,
}

// ✅ Fields are private; mutation is via explicit methods.
pub struct Vehicle {
    position: Vec3,
    velocity: Vec3,
}

impl Vehicle {
    pub fn position(&self) -> Vec3 { self.position }
    pub fn apply_acceleration(&mut self, a: Vec3, dt: f32) {
        self.velocity += a * dt;
        self.position += self.velocity * dt;
    }
}
```

**Rationale:** mutable public fields make refactoring impossible —
any code anywhere can set them. Explicit methods localize the
mutation surface.

### 3.4 Parsing Is Pure

Every `parse_*` function in `opts-core` must be pure:

- **No file I/O** — takes `&str` or `&[u8]`, returns `Result<T, E>`.
- **No global state.**
- **No logging side effects beyond `tracing` events.**
- **No mutation of inputs.**

The caller (in `opts-tools`, `opts-bevy`, or `opts-game`) is
responsible for reading the file. This makes parsers trivially
testable and side-effect-free.

```rust
// ✅ Correct signature for a parser.
pub fn parse_bus_file(input: &str) -> Result<BusFile, BusParseError>;

// ❌ Wrong: parser does I/O.
pub fn parse_bus_file(path: &Path) -> Result<BusFile, BusParseError>;
```

The second form is convenient but unfriendly. Provide a separate
`load_bus_file(path)` helper that reads the file and delegates.

### 3.5 The Three-Layer Parser Pattern

Every format parser in OPTS follows this structure:

1. **Lexing** — break raw input into tokens (for text) or bytes (for
   binary).
2. **Parsing** — turn tokens into an AST or a flat struct.
3. **Validation** — check invariants and normalize into a
   domain-friendly form.

Each layer is a separate module with its own tests. Do not skip
layers. Do not combine layers into one function.

```
opts_core::formats::o3d
├── lexer.rs    // bytes → chunks
├── parser.rs   // chunks → RawO3d
└── validate.rs // RawO3d → O3d
```

### 3.6 Structure Files by Concern, Not by Type

Do not create `models.rs`, `structs.rs`, or `types.rs`. These are
dump files that grow without bound.

Instead, group by domain concept:

```
opts-core/src/formats/
├── mod.rs
├── o3d/
│   ├── mod.rs
│   ├── header.rs      # the o3d header
│   ├── chunks.rs      # chunk parsing
│   ├── vertex.rs      # vertex buffer decoding
│   ├── material.rs    # material table
│   └── error.rs       # error type for this format
├── bus/
│   ├── mod.rs
│   ├── keyword.rs
│   ├── block.rs
│   └── error.rs
└── sco/
    └── ...
```

Each file is under 500 lines and has a single concern.

### 3.7 The 500-Line Rule

**No `.rs` file exceeds 500 lines of code**, excluding doc comments
and blank lines.

**Why 500?** Because it is roughly the point at which a reader can
no longer keep the file's contents in their head. Files under 500
lines can be read in one sitting; files over 500 require scrolling
and paging.

**When a file hits 500 lines:**

1. Identify the two or three distinct concerns in the file.
2. Split into submodules under a directory of the same name.
3. Update `mod.rs` to re-export public items.

**Exception:** generated code, test fixtures, and long lookup tables.
These must be clearly marked as such and excluded from the count via
`grep -v` in the lint tool.

### 3.8 Module Documentation

Every module starts with a `//!` doc comment:

```rust
//! OMSI `.o3d` mesh file parser.
//!
//! This module implements the three-layer parsing pipeline described
//! in `docs/src/formats/binary/o3d.md`. It parses the binary layout,
//! extracts the vertex and index buffers, and produces a normalized
//! [`O3d`] structure suitable for use by `opts-bevy`.
//!
//! # Examples
//!
//! ```
//! use opts_core::formats::o3d::parse;
//! let o3d = parse(&std::fs::read("model.o3d")?)?;
//! println!("{} vertices", o3d.vertex_count());
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```
```

The module doc answers: **what is this module for, and how do I use
it?** If it can't answer those questions, the module has no clear
reason to exist.

### 3.9 Public API Minimalism

Every public item is a commitment. Minimize the surface.

- **Start private.** Promote to `pub` only when needed.
- **Use `pub(crate)` for internal sharing** across modules in the
  same crate.
- **Do not re-export dependencies.** If `opts-core` uses
  `glam::Vec3`, do not `pub use glam::Vec3`. Let consumers import
  it from `glam`.
- **Use `#[doc(hidden)]` sparingly.** If an item needs to be public
  but undocumented, that is a smell.

### 3.10 Testing Strategy

Every crate has tests. Test placement:

| Test type | Location |
| :--- | :--- |
| Unit tests | Same file, `#[cfg(test)] mod tests` |
| Integration tests | `tests/` directory at crate root |
| Doc tests | In rustdoc `///` blocks |
| Property tests | `tests/` with `proptest` or `quickcheck` |

**Required test coverage per parser:**

- **Happy path** — a valid file parses correctly.
- **Truncated input** — a file cut off mid-stream fails cleanly.
- **Invalid bytes** — arbitrary garbage does not panic.
- **Count mismatch** — declared counts disagree with actual content;
  parser logs and proceeds.
- **Unknown keywords** — parser stores them in `unknown_fields`
  without failing.

Round-trip tests are required wherever serialization exists:
`parse(serialize(x)) == x`.

### 3.11 Documentation Standards

Code comments in OPTS follow these rules:

| Item | Required |
| :--- | :--- |
| Crate | `//!` at top of `lib.rs` |
| Module | `//!` at top of every module file |
| Public function | `///` with `# Arguments`, `# Returns`, `# Errors` |
| Public struct | `///` with field-level docs |
| Public enum | `///` with variant-level docs |
| Public trait | `///` with contract description |
| Private function | `///` only if non-obvious |
| Inline comment | Only for "why", never for "what" |

The crate-level doc lives in `docs/crates/<crate>.md`. The
module-level doc lives in the source. The two are not duplicates —
the crate doc explains the crate's role in the system, the module
doc explains the module's role in the crate.

### 3.12 Anti-Patterns to Avoid

A short, non-exhaustive list of things OPTS does **not** do:

| ❌ Anti-pattern | ✅ Alternative |
| :--- | :--- |
| OOP with structs + methods as the primary abstraction | Data structs + free functions |
| `Rc<RefCell<T>>` for shared mutable state | Ownership at the top, borrows below |
| Public mutable fields | Private fields + accessor methods |
| `unwrap()` in library code | `Result` with domain errors |
| `Box<dyn Error>` in public APIs | Typed errors with `thiserror` |
| `String` parameters when `&str` works | Borrowed parameters |
| `.clone()` to silence the borrow checker | Restructure ownership |
| Generic returns where concrete works | Concrete types unless generic is needed |
| Traits with one implementor | Free functions |
| Deeply nested modules (`a::b::c::d::e::f`) | Shallower hierarchy |
| God structs with 20+ fields | Split by concern |
| Circular crate dependencies | Restructure or introduce a shared crate |
| `unsafe` without `// SAFETY:` comment | Document, test, or avoid |
| `println!` in library code | `tracing` events |
| Files over 500 lines | Split into submodules |
| Functions over 80 lines | Extract helpers |

---

## 🔧 Tooling

### Required

The following tools are enforced by CI and must be run before
committing:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

`cargo clippy -D warnings` means **every clippy warning is an error**.
There are no exceptions. If a lint is wrong for a specific case,
use `#[allow(...)]` with a comment explaining why.

### Recommended

- **`cargo nextest`** — faster test runner than `cargo test`.
- **`cargo deny`** — dependency licensing and security audit.
- **`cargo spellcheck`** — catches typos in docs.
- **`typos`** — fast typo detection across the repo.
- **`cargo udeps`** — finds unused dependencies.
- **`cargo bloat`** — analyzes binary size (for `opts-game`).

### Forbidden

- **`unsafe` without review.**
- **`#[allow(clippy::all)]`** — disables everything. Use targeted
  allows.
- **`#[allow(dead_code)]`** — delete the code, or make it public.
- **`#[allow(unused_imports)]`** — remove the import.
- **`#![allow(...)]` at crate level** — always narrow to the specific
  item.

---

## 📋 Checklist

Before submitting a pull request, verify:

- [ ] `cargo fmt --all` applied.
- [ ] `cargo clippy -- -D warnings` passes.
- [ ] `cargo test --workspace --all-features` passes.
- [ ] Every new public item has rustdoc.
- [ ] Every new module has a `//!` doc comment.
- [ ] Every new file is under 500 lines.
- [ ] Every new function is under 80 lines.
- [ ] No `unwrap()` in library code paths.
- [ ] No `println!` in library code.
- [ ] No `unsafe` without `// SAFETY:` and reviewer sign-off.
- [ ] No crate boundary violations (`opts-core` doesn't import Bevy).
- [ ] Tests cover the happy path, the error path, and one
      malformed-input case.
- [ ] If a spec exists, the code follows the spec.
- [ ] If the spec is wrong, the spec was updated (not the code
      made to match wrong behavior).
- [ ] Commit message follows `<type>(<scope>): <summary> (M-NNNN)`.

---

## 🔗 Related Documents

- [`CONTRIBUTING.md`](../../../CONTRIBUTING.md) — Contribution workflow
- [`dev/PROMPTS.md`](../../dev/PROMPTS.md) — Dev agent prompt library
- [`docs/src/adr/0001-why-bevy-and-rust.md`](../adr/0001-why-bevy-and-rust.md) — Technology choices
- [`docs/src/adr/0002-clean-room-policy.md`](../adr/0002-clean-room-policy.md) — Legal constraints
- [`docs/src/glossary.md`](../glossary.md) — Shared vocabulary

---

## 📝 Changelog

- `2026-10-04` — Initial version. Three-part structure: universal
  principles, Rust practices, OPTS design conventions.