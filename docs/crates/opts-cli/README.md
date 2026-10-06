# `opts-cli`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L6 · Application |
| **Role** | composition root |
| **Kind** | lib → bin (`opts`) |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Becomes the `opts` binary |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

The command-line front end: `opts inspect`, `opts convert`, `opts validate`, `opts graph`. Argument parsing and output formatting only; the commands themselves live in `opts-tools`.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

```text
opts inspect  <file>                 # pretty-print parsed AST + diagnostics
opts validate <omsi-root> [--format] # parse everything, report problems
opts graph    <omsi-root> <asset>    # dependency graph via opts-discovery
opts convert  <file> --to <fmt>      # e.g. .o3d → glTF (later)
```

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-tools` *(planned)* | Command implementations |
| `argh` **or** `clap` | Argument parsing — pick one workspace-wide |
| `anyhow`, `tracing-subscriber` | Binary-level errors and logging |

Currently declares parser libraries (`nom`, `binrw`, `glam`, `petgraph`) — those belong to lower layers. ⚠️ `argh` is not declared in `[workspace.dependencies]` (Audit F2).

## 🧭 Design Decisions

- Proposed split (Architecture Q2): **`opts-tools` = library** of commands, **`opts-cli` = binary**. This lets the editor and tests call the same commands without spawning a process.
- Alternative: drop `opts-cli` entirely and keep `opts-tools` as both. Simpler, but loses reuse.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- ⚠️ Undeclared workspace dependency `argh`.

## 🔮 Future Direction

Shell completions, JSON output mode for CI pipelines.

## 🔗 Cross-References

- [opts-tools](../opts-tools/)
- [Workspace Audit F2, F6](../audit.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
