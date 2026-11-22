# Scripting: options for OPTS

| Metadata | Value |
| :--- | :--- |
| **Status** | 🔬 Research and proposal. The owner decides. |
| **Date** | 2026-11-22 |
| **Relates to** | [M-0092](../src/roadmap/phase-6/M-0092.md), [M-0026](../src/roadmap/phase-4/M-0026.md), [M-0172](../src/roadmap/phase-18/M-0172.md), [M-0173](../src/roadmap/phase-18/M-0173.md), [ADR-0007](../src/adr/0007-hot-reload.md) section 9, [ADR-0002](../src/adr/0002-clean-room-policy.md) |

## 1. The question

OPTS already runs OMSI's own vehicle script language, `.osc`, in a VM written in Rust ([script VM](../src/architecture/script-vm.md), [`.osc` format](../src/formats/text/vehicles/osc.md)). That VM is wired to vehicles only; scenery scripts do not run yet ([scenery benchmark](../src/architecture/scenery-benchmark.md)). The vision asks for "a scripting layer for vehicles and scenarios" ([vision](../src/vision.md)).

Three things need scripting that `.osc` does not serve well:

* **New content** written for OPTS, which does not need OMSI compatibility.
* **Scenarios**: a goal, a start, a timetable and a scoring rule ([M-0172](../src/roadmap/phase-18/M-0172.md)).
* **Roaming services and traffic events**: when and where emergency vehicles appear ([M-0173](../src/roadmap/phase-18/M-0173.md)).

The question is which of four options OPTS takes. This file does not decide it.

## 2. The four options

| Option | Summary |
| :--- | :--- |
| **(a) OMSI compatibility only** | Keep `.osc` as the only script language. New content and scenarios use data files (TOML or the OMSI keyword format) plus the existing callbacks. |
| **(b) OPTS-native DSL** | A new small language or declarative format written for OPTS, with its own parser and semantics. It can be declarative (rules, no loops) or imperative. |
| **(c) Embed an existing language** | Lua, Rhai or WebAssembly, run inside the simulation through a host API. |
| **(d) Do nothing beyond OSC** | The same as (a), but stated as a deliberate end point: scenarios are hand-coded in Rust, and no content-facing language is added. |

Options (a) and (d) differ only in intent. Option (a) leaves room for later work; option (d) closes the question.

## 3. What each option gives OPTS as a simulation

| Option | Gives | Costs | Who can write it | Main risks |
| :--- | :--- | :--- | :--- | :--- |
| **(a)** | Full OMSI compatibility for vehicles. Nothing new to learn for OMSI creators. Existing VM, rig and tests. | No general logic for scenarios. Declarative scenario data must be invented separately, so the format is designed twice. The OMSI language is stack-based and hard to write ([`.osc`](../src/formats/text/vehicles/osc.md)). | Only people who know the OMSI stack language. | Every new mechanic needs a Rust change. Content creators stay dependent on contributors. |
| **(b)** | A format designed for scenarios, challenges and roaming services. Can be validated fully before it runs (the editor can check every rule). No loops keeps runtime cost and termination simple. | A new parser, a new schema, documentation, and a test rig. Save compatibility: scenario files must carry a format version ([ADR-0011](../src/adr/0011-session-save-format.md) gives a pattern). A second language to learn. | Creators who can write TOML or rules; the editor can write it for them. | Scope creep toward a general language, which is option (c) by another name. A rule set that cannot express a case forces a Rust change. |
| **(c)** | A mature language with tooling, a debugger in the wider ecosystem, and a large pool of people who already know it. Scripts can be hot reloaded with state migration. | A new dependency and its licence. A sandbox that must be configured and tested. Determinism: a replay or a save that depends on a script must produce the same result on every platform. Documentation of the host API. A new attack surface when scripts come from the community. | Anyone who knows the chosen language. | Safety of user scripts (below). Non-deterministic behaviour (below). A dependency that is hard to replace once content depends on it. |
| **(d)** | No new language, no new parser, no sandbox to build. Simplest to test. | Scenarios and challenges are written by contributors only. The vision's "modders publish … scenarios" and "a scripting layer for scenarios" are not met. | Contributors with Rust. | The vision text stays partly unmet. The scenario editor (M-0172) would have nothing to edit but static data. |

## 4. Details for the options that are not free

### 4.1 Sandboxing and resource limits

Rhai documents its safety model. Its safety page names the threats it defends against for scripts from untrusted sources: unbounded memory growth, infinite loops, deep recursion, arithmetic faults, unwanted file imports and access to sensitive data. It has feature flags, `internals` (exposes host internals, which the docs call a bad idea outside trusted code) and `unchecked` (disables safety checks). The page states that the engine aims never to panic on script behaviour. It does not list the configuration names for operation or depth limits on that page; those are on the sandbox chapter that the page links to, which was not read in this pass ([sources](sources.md), S-03).

The part of the Lua 5.4 manual that was read describes an environment for a loaded chunk: `load` and `loadfile` can give a chunk an environment other than the global one, so the chunk sees only the functions the host puts in it (§2.2, [Lua 5.4 manual](https://www.lua.org/manual/5.4/manual.html), S-04). The same section and §2.5 note that garbage collector behaviour is not portable across platforms and releases. The read did not reach the debug library, so an instruction-count limit in Lua was not confirmed and is not claimed here.

Wasmtime is built for untrusted code. Its security page states that running untrusted code safely inside a sandbox is a main goal, and that filesystem access follows WASI's capability model, so a module reaches only the directories it is granted (S-05). Its deterministic-execution page recommends fuel-based interruption and memory limits (S-06).

**Relevance to OPTS.** The control server is already the weakest point: its blocker ISS-001 is an empty or weak token accepted for `auth`, and ISS-002 and ISS-003 cover loopback without a token and missing timeouts ([issues register](../audit/issues-2026-11-18.md)). Any option that lets a script load another script or a file through `reload.asset` must come after those fixes. The vehicle VM already has no loops and limited nesting, which is a sandbox by construction ([script VM](../src/architecture/script-vm.md)).

### 4.2 Determinism and saves

OPTS needs replays and saves to reproduce ([script VM](../src/architecture/script-vm.md): seeded random, fixed-index variable names). An embedded language that holds state needs the same rules: a fixed seed, no wall-clock reads, and a state format in the save. [ADR-0011](../src/adr/0011-session-save-format.md) already defines a section registry, and a script's state would be one section. Wasmtime's guidance is concrete: NaN bit patterns are unspecified and canonicalising them slows floating-point code, and memory growth can fail non-deterministically unless limited (S-06). Floating-point behaviour matters for a driving simulation, so option (c) costs speed as well as design time.

### 4.3 Test rigs

The `.osc` route already has a headless rig (`script_rig`), which the script generator would reuse ([script generator §7](../src/architecture/script-generator.md)). Option (b) would need its own rig. Option (c) would need one per language binding. [M-0092](../src/roadmap/phase-6/M-0092.md) is the place to decide whether the rig is shared.

### 4.4 Clean-room exposure

[ADR-0002](../src/adr/0002-clean-room-policy.md) applies to every option. The `.osc` language is documented from public material and from the project's measurements; no OMSI script text is copied into new templates ([script generator §2](../src/architecture/script-generator.md)). Options (b) and (c) do not touch OMSI at all, so their clean-room exposure is lower. The exposure is in the *content*: if a generated scenario reproduces an OMSI or LOTUS scenario, the policy applies to that content.

Community summaries of OMSI's map editor manual exist (for example a 2011 manual on Scribd and a Japanese translation on a community wiki). They were found by search and not read in full for this file ([sources](sources.md), S-07, S-08). Their use as reference is owner decision 13 in the [README](README.md).

## 5. What the scenario editor and roaming services need from the scripting layer

[M-0172](../src/roadmap/phase-18/M-0172.md) needs:

* A **scenario file** with the map, the start time and position, the vehicle, the timetable reference and the goal. Declarative.
* A **rule set** for challenges: punctuality against the timetable and passengers carried ([ISS-048](../dev/issues/ISS-048-challenges-and-scoring.md)). These rules read data OPTS already models: timetable arrival times and passenger counts ([M-0101](../src/roadmap/phase-4/M-0101.md)). A declarative rule language is enough.
* A **scoring output**: a session summary or a stored result (owner decision 2 in the [README](README.md)).
* An **event hook** for scenario steps (a stop reached, a time passed). Simple triggers are enough for a first version.

[M-0173](../src/roadmap/phase-18/M-0173.md) needs:

* A **spawn rule**: which group, at what time or density, on which road. The `global.cfg` and `ailists.cfg` files already name groups ([M-0173 scope](../src/roadmap/phase-18/M-0173.md)); a rule adds timing.
* A **priority rule** for sirens, which [M-0049](../src/roadmap/phase-4/M-0049.md) and [M-0111](../src/roadmap/phase-11/M-0111.md) own. It needs no general language.

Both needs are declarative. Neither needs a loop or a function. That is the strongest argument for option (b) in a narrow form.

## 6. Recommendation (a proposal)

**Keep `.osc` for vehicles (option a). For scenarios and roaming rules, add a small declarative OPTS format (a narrow form of option b) with no loops and no user-defined functions. Defer an embedded general language (option c) until a concrete need appears.**

Reasoning:

1. The scenario and roaming needs are declarative (section 5). A language with loops is more than they need, and it adds the sandbox, determinism and documentation costs of section 3 without a use case.
2. A declarative format can be validated completely before it runs. The editor's validation engine ([validation](../src/editor/validation.md)) can check every rule, which is what creators need.
3. It does not close the door on (c). If the format needs a new expression, the expression grammar can grow into a small language; the choice of (c) then becomes a question about one feature.
4. It avoids a new attack surface while ISS-001 to ISS-003 are open.

**Cost of the recommendation.** It adds a parser and a schema, and a second file kind that creators must learn. These are estimated but not sized in this pass.

## 7. Conditions that would change the recommendation

| Condition | Effect |
| :--- | :--- |
| A scenario needs branching logic that the rule set cannot express, after two real scenarios are written | Move to option (c) for scenarios, starting with the smallest embeddable language that meets the sandbox rules in section 4.1 |
| Community authors ask for vehicle logic outside the OMSI stack language | Option (c) for new vehicles, with `.osc` kept for OMSI content |
| The OMSI script language is shown to be unnecessary for OPTS content, and OPTS content is written only in OPTS | Option (b) becomes the main format, and the OMSI VM serves only imported content |
| ISS-001 to ISS-003 remain open at the time a scripting host is built | Do not add option (c) until they close, since a script host widens what a loaded file can do |
| The owner decides that scenarios are a play-only feature with no community authoring | Option (d) for now; option (b) can wait |

## 8. Open questions

* **Q-SD-1.** Does a scenario file belong in the save format ([ADR-0011](../src/adr/0011-session-save-format.md)), or in its own format? Owner decision 2.
* **Q-SD-2.** Does a declarative rule need to load in OMSI, or is it OPTS-only? Owner decision 4 and 5.
* **Q-SD-3.** What is the minimum set of triggers for scenario steps? Not specified; the first slice of M-0172 must decide.
* **Q-SD-4.** The Rhai configuration names for operation and depth limits were not read. Read the sandbox chapter before any choice is made.
* **Q-SD-5.** The per-consist budget that [osc-runtime](../src/architecture/osc-runtime.md) describes was not found in code by `grep`. Its status decides how much of the existing VM can be called a sandbox.
