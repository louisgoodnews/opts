# ISS-063: The crate-graph rule that only three crates may depend on Bevy

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-063 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (architecture decision) |
| **Milestone** | none; owner decision needed before any code |
| **Status** | decided 2026-11-25 by the owner: option 1 for the shared game-state crate, which may depend on Bevy directly (approved). Rule 4 amendment to name the exempt crates is still to be written |

## Problem

The crate graph (`docs/src/architecture/crate-graph.md`, rule 4) says that only `opts-bevy`,
`opts-gui` and `opts-game` may declare the `bevy` dependency. The split work of ISS-061 and ISS-062
creates crates that need Bevy types (`Component`, `Resource`, `Handle<Image>`, the `Transform` of a
moved module), so they break the rule:

- `opts-scene-trace` and `opts-spatial-zones` (earlier split, commit `87bf161b` on the M-0103 branch)
- `opts-light-quality`, `opts-audio`, `opts-main-camera` (ISS-061 branch)

The rule's intent (a small set of crates touches the engine) is sound, but splitting the crates that
hold Bevy-facing code needs a decision: amend the rule, or route Bevy through one facade crate.

## Options for the owner

1. **Amend rule 4.** Allow a leaf crate to depend on `bevy` when it holds Bevy-facing types or systems
   and nothing else of the engine. State the list of such crates in the rule. Simplest; the rule then
   documents what is already true.
2. **One facade crate.** A single crate (for example `opts-bevy-types`) declares `bevy` and re-exports
   what the leaf crates need; the leaf crates depend on the facade, not on `bevy`. This keeps the
   literal rule, but the facade is itself a Bevy dependency with a wider surface, and the leaf crates
   still compile against Bevy's types. It moves the exception into a crate instead of removing it.
3. **Keep the Bevy-facing code in `opts-bevy`.** Revert the split of the Bevy-facing modules; keep only
   the pure modules (no `bevy` in their code) out. This needs the budget room from elsewhere.

## Recommendation (not decided)

Option 1, with the list of exempt crates written into rule 4 and the reason (Bevy types only, no
engine logic). Option 2 adds a crate whose only job is to be an exception; it hides the dependency
rather than stating it.

## Next

1. The owner chooses an option and amends rule 4 in `docs/src/architecture/crate-graph.md` (the rule
   text and the list). No code changes before that.
2. If option 2 or 3: a follow-up issue for the facade crate or the revert, with its own budget and
   merge plan.
3. Then the ISS-061 branch can be committed and merged, with its crate-graph rows checked against the
   amended rule (`scripts/crate_graph.py --check` checks the table, not the rule text).

## Affected branches

- `agent/iss-061-split-opts-bevy` (uncommitted, Cargo.lock regenerated with `cargo metadata`)
- `agent/m0103-ai-inputs` commit `87bf161b` (scene trace and spatial zones)

## Owner decision (2026-11-25)

Approved: the shared game-state crate `opts-game-state` (ISS-064 stage 2) depends on `bevy` directly, because its types derive Bevy traits. Rule 4 is to be amended to list the exempt crates (Bevy-facing types only, no engine logic). The amendment itself is a docs change and is not yet made.
