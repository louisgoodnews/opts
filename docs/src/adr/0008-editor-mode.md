# ADR-0008: Editor mode inside the game executable, with overlay projects

* **Status:** Proposed (2026-10-11)
* **Context:** OPTS can play OMSI content. Users also need to repair, adapt and create content (maps, vehicles, timetables …). OMSI's own editor is closed; community tools are fragmented.
* **Decision:**
  1. The editor is an **application state** of `opts-game` (`AppState::Editor`, `--editor`) backed by two new crates: `opts-edit` (pure documents, commands, schema, validation, project) and `opts-editor` (Bevy tools and panels). `opts-core` gains writers and a lossless text layer.
  2. The viewport is the **game renderer**; the editor adds overlays only.
  3. Projects are **overlays** over a read-only OMSI installation; the installation is never modified except by an explicit, previewed *Install into OMSI* action with receipts.
  4. Files round-trip **losslessly**; unknown fields are preserved; the schema registry is generated against the specifications.
  5. **Protected (scrambled) meshes are read-only** and are never written or exported; new meshes are written plain.
  6. Everything the editor does is a **command**, scriptable through the control protocol (`editor.*`).
* **Consequences:** + one renderer, one set of loaders; + safe by construction; + testable headlessly; − the loaders must be refactored to rebuild single elements (M-0046); − every writer needs OMSI acceptance checks that cannot be automated (no decompilation); − larger workspace (two crates).
* **Alternatives considered:** a separate editor binary (duplicates the renderer and loaders); editing files in place (violates P1); a database-backed document store (loses file fidelity and Git-friendliness); web-based editor (cannot share the Bevy renderer).
* **Specification:** [Editor mode](../editor/README.md).
