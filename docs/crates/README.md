# 📦 OPTS Crate Book

| Metadata | Value |
| :--- | :--- |
| **Scope** | Every crate in `crates/` — current and planned |
| **Status** | 🟡 Proposed architecture — see [ADR-0003](adr/0003-layered-crate-architecture.md) |
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

| Crate | Kind | Proposed layer | Proposed fate |
| :--- | :--- | :--- | :--- |
| [`opts-utils`](opts-utils/) | lib | Foundation | ✅ Keep — strictly minimal |
| [`opts-core`](opts-core/) | lib | Model | 🔁 Repurpose — domain model only, parsers move out |
| [`opts-parsers`](opts-parsers/) | lib | Formats | 🔁 Becomes the `opts-formats` facade over `opts-fmt-*` |
| [`opts-discovery`](opts-discovery/) | lib | Pipeline | ✅ Keep |
| [`opts-assets`](opts-assets/) | lib | Pipeline | 🔁 Repurpose — Bevy-free asset catalog |
| [`opts-sim`](opts-sim/) | lib | Simulation | ✅ Keep — becomes facade over `opts-sim-*` |
| [`opts-bevy`](opts-bevy/) | lib | Engine | ✅ Keep — becomes the `OptsPlugins` group |
| [`opts-graphics`](opts-graphics/) | bin ⚠️ | Engine | 🔁 Convert to lib → `opts-bevy-render` |
| [`opts-gui`](opts-gui/) | lib | Engine | 🔁 Rename → `opts-bevy-ui` |
| [`opts-game`](opts-game/) | bin | Application | ✅ Keep |
| [`opts-cli`](opts-cli/) | lib | Application | 🔁 Becomes the `opts` binary |
| [`opts-tools`](opts-tools/) | bin | Application | 🔁 Becomes the tool-command library |
| [`opts-benchmark`](opts-benchmark/) | lib | Development | 🔁 Dev-only, `publish = false` |

Legend: ✅ keep as-is · 🔁 rename / repurpose / restructure · 🆕 planned · 💭 speculative · ⚠️ currently broken or rule-violating

> [!NOTE]
> All crates are empty skeletons today. That is the best moment to fix the crate graph — it costs nothing to move code that does not exist yet.

---

## 🔗 Links Between the Two Books

Links from this book into the main book point at the **published** site, because the Crate Book is deployed under `<site>/crates/` next to the main book. Use `../formats/text/sco.html` from top-level pages and `../../formats/text/sco.html` from `<crate>/README.md` and `adr/` pages. These links do not resolve in a local file browser.

---

## ✍️ Updating This Book

Per [PROMPTS.md](https://github.com/louisgoodnews/opts/blob/main/docs/dev/PROMPTS.md), every change that touches a crate updates that crate's page. Use the template in [Crate Conventions § Crate Page Template](conventions.md#-crate-page-template). New crates get a new folder `docs/crates/<crate>/README.md` and a `SUMMARY.md` entry in their layer.
