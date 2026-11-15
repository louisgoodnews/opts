# 📦 OPTS Crate Book

| Metadata | Value |
| :--- | :--- |
| **Scope** | Every crate in `crates/` — current and planned |
| **Status** | 🟡 Proposed architecture — see [ADR-0003](../src/adr/0003-layered-crate-architecture.md) |
| **Last Updated** | 2026-10-04 |

---

## 📖 What This Book Is

The [main book](../) describes *what* OPTS reads and simulates: formats, roadmap, policy.
This book describes *how the code is cut*: which crate owns what, which crate may depend on which, and why.

It complements rustdoc rather than duplicating it:

| Question | Where it is answered |
| :--- | :--- |
| What does `parse_sco` return? | rustdoc (`///` in the source) |
| What does the `formats` module do inside its crate? | rustdoc (`//!` in the module) |
| Why does this crate exist, and what may it depend on? | **This book** — `docs/crates/<crate>/README.md` |
| How do all crates fit together? | **This book** — [Proposed Architecture](architecture.md) |

---

## 🧭 How to Read This Book

1. **[Proposed Architecture](architecture.md)** — the layered model, dependency rules, extension seams. Start here.
2. **[Crate Catalog](catalog.md)** — every crate, current and future, with its layer and the roadmap phase that creates it.
3. **[Crate Conventions](conventions.md)** — naming, features, errors, manifest metadata, the per-crate doc template.
4. **[Workspace Audit & Migration Plan](audit.md)** — what the workspace looks like today, what is broken, and the path to the target.
5. **Per-crate pages** — one page per existing crate.

---

## 🗂️ Current Crates at a Glance

> [!WARNING]
> This table is the **2026-10-04 proposal** and is no longer the workspace state. The five crates it listed as `opts-utils`, `opts-parsers`, `opts-discovery`, `opts-assets` and `opts-graphics` were removed and their pages deleted. For the live crate graph see [Crate Graph](../src/architecture/crate-graph.md) and the [Crate Catalog](catalog.md).

| Crate | Kind | Proposed layer | Proposed fate |
| :--- | :--- | :--- | :--- |
| [`opts-core`](opts-core/) | lib | Model | 🔁 Repurpose — domain model only, parsers move out |
| [`opts-sim`](opts-sim/) | lib | Simulation | ✅ Keep — becomes facade over `opts-sim-*` |
| [`opts-bevy`](opts-bevy/) | lib | Engine | ✅ Keep — becomes the `OptsPlugins` group |
| [`opts-gui`](opts-gui/) | lib | Engine | 🔁 Rename → `opts-bevy-ui` |
| [`opts-game`](opts-game/) | bin | Application | ✅ Keep |
| [`opts-cli`](opts-cli/) | lib | Application | 🔁 Becomes the `opts` binary |
| [`opts-tools`](opts-tools/) | bin | Application | 🔁 Becomes the tool-command library |
| [`opts-benchmark`](opts-benchmark/) | lib | Development | 🔁 Dev-only, `publish = false` |

Legend: ✅ keep as-is · 🔁 rename / repurpose / restructure · 🆕 planned · 💭 speculative · ⚠️ currently broken or rule-violating

> [!NOTE]
> The statements about empty skeletons are historical (2026-10-04). The crate graph has since been restructured; see the catalog for the current state.

---

## 🔗 Links Between the Two Books

Links from this book into the main book point at the **published** site, because the Crate Book is deployed under `<site>/crates/` next to the main book. Use `../formats/text/sco.html` from top-level pages and `../../formats/text/sco.html` from `<crate>/README.md` and `adr/` pages. These links do not resolve in a local file browser.

---

## ✍️ Updating This Book

Per [PROMPTS.md](https://github.com/louisgoodnews/opts/blob/main/docs/dev/PROMPTS.md), every change that touches a crate updates that crate's page. Use the template in [Crate Conventions § Crate Page Template](conventions.md#-crate-page-template). New crates get a new folder `docs/crates/<crate>/README.md` and a `SUMMARY.md` entry in their layer.
