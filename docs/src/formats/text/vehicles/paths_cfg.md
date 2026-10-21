# `.cfg` — Model Paths Configuration Files

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions** | `.cfg` |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::paths_cfg` |
| **Encoding** | Windows-1252 (CP1252) |
| **Status** | 🟢 Specified — Ready for Implementation |
| **Last Updated** | 2026-10-03 |

---

## 📖 Overview

A **paths** `.cfg` file defines the **passenger path graph** for a vehicle segment. It is the spatial counterpart to the `passengercabin_cfg` file:

- The **paths file** defines pathpoints (graph nodes) and pathlinks (graph edges).
- The **passengercabin file** references pathpoints by index to place entries, exits, stampers, and ticket sale points.

Together they describe how passengers move through a vehicle from the moment they board until the moment they alight.

### Relationship to Other Files

| File | Relationship |
| :--- | :--- |
| `.bus` | References the paths `.cfg` via the `[paths]` keyword. |
| `passengercabin_cfg` | **Depends on this file.** All `[entry]`, `[exit]`, `[stamper]`, and `[ticket_sale]` declarations use pathpoint indices defined here. |
| `model_cfg` | Independent. Pathpoint coordinates are not required to match visible geometry. |
| `.o3d` | Not referenced directly. |

> 🎯 **Why this matters:** The path graph is the substrate for **passenger simulation**. Without it, `opts-sim` cannot path-find passengers from boarding to alighting. This is the foundation of the transit simulation's most visible behavior.

### Sample Corpus

This specification is derived from three sample files:

| File | Vehicle Segment | Pathpoints | Notable Traits |
| :--- | :--- | :--- | :--- |
| `paths_EN92.cfg` | Solobus (single segment) | 17 (0–16) | Rear pathpoint at y = −4.616 |
| `paths_GN92_main.cfg` | Articulated front segment | 17 (0–16) | Rear pathpoint at y = −4.331 (coupling point) |
| `paths_GN92_trail.cfg` | Articulated rear segment | 11 (0–10) | Front pathpoint 0 labeled `Übergang` (transition) |

**Key observation:** `paths_EN92.cfg` and `paths_GN92_main.cfg` are **identical from pathpoint 0 through 14**. They differ only at pathpoints 15 and 16, where the solobus extends further to the rear wall and the articulated main stops at the coupling joint.

**Key observation:** The `y` coordinate of the main segment's pathpoint 16 (`−4.331`) **exactly matches** the `[coupling_back]` y-coordinate in `MAN_GN92_main.bus`. This confirms that pathpoint 16 marks the articulation joint.

**Key observation:** The `y` coordinate of the trailer's pathpoint 0 (`3.784`) is offset by ≈ 0.4 m from the trailer's `[coupling_front]` y-coordinate (`4.169`) in `MAN_GN92_trail.bus`. This is consistent with the pathpoint being *just inside* the trailer, where passengers can stand, while the coupling is at the bumper.

---

## 🔤 General Syntax

Same rules as the other `.cfg` variants:

| Rule | Description |
| :--- | :--- |
| **Keywords begin a line** | `[` must be the first character. |
| **Keywords stand alone** | `]` must be the last character. |
| **Parameters follow directly** | One per line, immediately after the keyword. |
| **Everything else is a comment** | Including bare numbers and prose. |
| **Order matters** | Implicit indexing is used throughout. See [Implicit Indexing](#-implicit-indexing). |
| **Windows-1252 encoding** | Confirmed again via mojibake observation (`Tür` → `T黵` and `T�r`). |

---

## 🏗️ Document Structure

A paths file is divided into **four logical sections**, though the parser does not enforce section boundaries — they are conventional only:

| Section | Contains | Purpose |
| :--- | :--- | :--- |
| **Soundsets** | `[stepsoundpack]` | Footstep sound banks for passengers. |
| **Pathpoints** | `[pathpnt]` | Graph nodes (positions in 3D space). |
| **Metadata** | `[next_stepsound]`, `[next_roomheight]` | Global references. |
| **Links** | `[pathlink]` | Graph edges (directed connections between nodes). |

The parser processes the file **sequentially**, building up the pathpoints array as it goes. Pathlinks always reference already-declared pathpoints.

---

## 🔢 Implicit Indexing

Two keywords are **implicitly indexed** by their order of appearance:

| Keyword | Index namespace |
| :--- | :--- |
| `[stepsoundpack]` | Sound pack index. |
| `[pathpnt]` | **Pathpoint index.** This is the critical one. |

The index is **not written in the file as data**. It is purely positional. Authors typically write the index as a **comment** before each block for readability:

```
5 (Entwerter)
[pathpnt]
0
3.584
0.480

6
[pathpnt]
0
2.705
0.480
```

The bare numbers `5` and `6` are comments and are **ignored by the parser**. The parser must count `[pathpnt]` blocks in declaration order to assign indices.

> 🎯 **Critical for Parser:** The pathpoint index is the **primary key** of the path graph. It is what `passengercabin_cfg` references via `[entry]`, `[exit]`, `[stamper]`, and `[ticket_sale]`. Getting this indexing wrong will misalign the entire passenger simulation.
>
> **Verification checklist:**
> - The first `[pathpnt]` in the file is index 0.
> - The Nth `[pathpnt]` in the file is index N−1.
> - No gaps, no duplicates.

### ✅ Per-Segment Indexing

> **Resolved:** Pathpoint indices are **local to each vehicle segment**. Every segment's paths file **starts at index 0**, regardless of whether the segment is a standalone bus, an articulated front, or an articulated trailer.

This is directly confirmed by the sample corpus: the trailer's paths file (`paths_GN92_trail.cfg`) declares pathpoints from `0` to `10`, starting fresh. It does **not** continue the main segment's numbering.

| Concern | Implication |
| :--- | :--- |
| **Namespace** | Each segment owns its own pathpoint namespace. Index 0 in the main segment is a different pathpoint than index 0 in the trailer. |
| **Chain assembly** | When assembling an articulated vehicle, `opts-sim` must combine path graphs from each segment, preserving segment identity. A `ChainPathGraph` type with `(segment_index, pathpoint_index)` composite keys is required. |
| **Cross-segment links** | `[linkToPrevVeh]` and `[linkToNextVeh]` in the passengercabin file carry the cross-segment relationships. They resolve to pathpoint indices in the **neighboring segment's** namespace, not in a global one. |
| **Simulation pathfinding** | Passenger pathfinding operates on the combined graph. An entry in the main segment and an exit in the trailer are two nodes in one combined graph, connected via the cross-segment edges declared by the link keywords. |
| **Consistency** | Every paths file begins at 0. This is a hard invariant. A paths file that starts at 1 or skips indices is malformed. |

---

## 📚 Keyword Reference

### 🔊 Footstep Sound Packs

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[stepsoundpack]` | 1 + N | Line 1 is count N. The next N lines are `.wav` file paths. |

Sound packs are indexed by order of appearance. All three sample files declare the same pack:

```
[stepsoundpack]
5
Step_01.wav
Step_02.wav
Step_03.wav
Step_04.wav
Step_05.wav
```

> 📝 **Observation:** The `Normal` sound pack uses the same five footstep samples across the EN92, GN92_main, and GN92_trail. This suggests sound packs are a shared resource, possibly drawn from a common vehicle sound folder.

> ⚠️ **Open Question:** How are sound packs selected at runtime beyond `[next_stepsound]`? Is the selection script-driven, or static? **Needs verification.**

---

### 📍 Pathpoints

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[pathpnt]` | 3 | A graph node (passenger position). |

#### Parameter Layout

| Line | Field | Unit | Description |
| :--- | :--- | :--- | :--- |
| 1 | `x` | m | Lateral coordinate (positive = right). |
| 2 | `y` | m | Longitudinal coordinate (positive = forward). |
| 3 | `z` | m | Vertical coordinate (floor height at this node). |

Pathpoints use the **segment-local origin**: Y-forward, Z-up. Because indexing is per-segment, coordinates are also per-segment and must be transformed when the chain is assembled.

#### Pathpoint Semantics

Every pathpoint represents a **passable location** on the vehicle's interior floor. Pathpoints are placed at:

| Location | Example from Sample |
| :--- | :--- |
| **At doors** | `0 (Tür VV)`, `2 (Tür VH)`, `10 (Tür MV)`, `12 (Tür MH)` |
| **Inside the cabin** | `6`, `7`, `8`, `13`, `14`, `15`, `16` |
| **At ticketing equipment** | `1 (Kasse)`, `5 (Entwerter)` |
| **At articulation boundaries** | `16` (main), `0` (trailer) |
| **At the front/rear of the graph** | `0`, `16` (main); `0`, `10` (trailer) |

Authors annotate pathpoints with German comments naming their purpose. These comments are **not part of the format** — they are documentation for human readers.

#### Notable Pathpoints in the Main Segment Corpus

Both the EN92 and GN92_main paths files share the same topology from 0 to 14:

| Index | Comment | Purpose |
| :--- | :--- | :--- |
| 0 | `Tür VV` | Front-front door (front half of the front door) |
| 1 | `Kasse` | Ticket sales location |
| 2 | `Tür VH` | Front-rear door (rear half of the front door) |
| 3 | `vor Pendel` | In front of the articulation bellows |
| 4 | `hinter Pendel` | Behind the articulation bellows |
| 5 | `Entwerter` | Ticket validation machine |
| 10 | `Tür MV` | Middle-front door |
| 12 | `Tür MH` | Middle-rear door |

> 📝 **Observation:** The comments `vor Pendel` and `hinter Pendel` at pathpoints 3 and 4 are inherited from the articulated design, even in the solobus EN92 file. These comments describe the *purpose* of the pathpoint, not the physical reality of the specific vehicle. The EN92 solobus has no bellows, but its path graph was derived from a template that expected one.

#### Divergence Between EN92 and GN92_main

At pathpoints 15 and 16, the two files diverge:

| Index | EN92 (solobus) | GN92_main (articulated front) |
| :--- | :--- | :--- |
| 15 | `(0.000, −3.155, 0.684)` | `(0.000, −3.408, 0.632)` |
| 16 | `(0.000, −4.616, 0.756)` | `(0.000, −4.331, 0.632)` |

Interpretation:

- **EN92** extends its walkable path all the way to the rear wall of the vehicle (`y = −4.616`) with a slight step up at the end (`z = 0.684 → 0.756`), reflecting the raised rear bench or engine hump typical of a solobus.
- **GN92_main** stops its walkable path at the articulation joint (`y = −4.331`), with a flat floor (`z = 0.632`), because beyond that point the trailer segment begins its own graph.
- **The `y = −4.331` value exactly matches** the `[coupling_back]` y-coordinate declared in `MAN_GN92_main.bus`. This is the smoking gun that confirms pathpoint 16 marks the physical articulation point.

#### Pathpoints in the Trailer Segment

| Index | Comment | Purpose |
| :--- | :--- | :--- |
| 0 | `Übergang` | Transition to the main vehicle (articulation joint) |
| 1 | — | Cabin traversal |
| 2 | — | Cabin traversal (branch point) |
| 3 | — | Door approach (left/right split) |
| 4 | `Tür HV` | Door, rear-front (front half) |
| 5 | — | Cabin traversal (branch point) |
| 6 | — | Door approach |
| 7 | `Tür HH` | Door, rear-rear (rear half) |
| 8 | — | Cabin traversal |
| 9 | — | Cabin traversal |
| 10 | — | Rear of trailer |

The trailer's path graph is deliberately shorter (11 nodes vs. 17), reflecting the trailer's simpler cabin layout and the absence of a driver position.

The `Übergang` label at pathpoint 0 is direct evidence that pathpoints at articulation joints are conceptualized as "transitions" rather than as ordinary cabin positions.

---

### 🔗 Pathlinks

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[pathlink]` | 2 | A directed edge between two pathpoints. |

#### Parameter Layout

| Line | Field | Description |
| :--- | :--- | :--- |
| 1 | `from` | Source pathpoint index (in this segment's namespace). |
| 2 | `to` | Destination pathpoint index (in this segment's namespace). |

Example:

```
[pathlink]
0
1
```

This means: "Pathpoint 0 connects to pathpoint 1." Both indices refer to pathpoints **within the same segment**.

#### Graph Topology

The links form a **directed graph**. Edges are one-way: if `0 → 1` is declared, the reverse requires an explicit `1 → 0` link.

**Main segment graph (EN92 and GN92_main share the same links):**

```
0 ── 1 ── 3 ── 4 ── 5 ── 6 ── 7 ── 8 ── 9 ─┬─ 10
                                          └─ 11 ─┬─ 12
                                                 └─ 13 ── 14 ── 15 ── 16
2 ──────────────────────────── 3
```

**Trailer segment graph:**

```
0 ── 1 ── 2 ─┬─ 3 ── 4
             └─ 5 ─┬─ 6 ── 7
                   └─ 8 ── 9 ── 10
```

> 📝 **Observation:** The trailer's pathpoint 0 (`Übergang`) is the sole entry into the trailer's graph. Passengers entering the trailer from the main segment arrive at trailer pathpoint 0, and from there follow the graph toward the rear. This is the physical analogue of boarding through an articulation joint.

> 📝 **Observation:** The main segment's path graph is **not symmetric**. `[pathlink] 0 1` exists, but `[pathlink] 1 0` does not. Passengers must walk **from front to back** through the cabin. This is consistent with how a real bus operates: passengers board at the front, validate their tickets mid-cabin, and alight at the middle or rear doors.

> ⚠️ **Open Question — Critical:** Does OMSI treat pathlinks as **directed** or **undirected**? The declaration syntax implies directed (`from`, `to`), and the asymmetric main-segment graph strongly suggests directed. However, OMSI's simulation might still traverse them bidirectionally.
>
> **Parser directive:** Store pathlinks as directed edges. Do not symmetrize in `opts-core`. Symmetrize (or not) in `opts-sim` after testing against observed OMSI behavior.

---

### 🌐 Global Metadata

| Keyword | Parameters | Description |
| :--- | :--- | :--- |
| `[next_stepsound]` | 1 | Default footstep sound pack index. |
| `[next_roomheight]` | 1 | Interior ceiling height reference. |

Both are present in all three sample files with identical values.

#### `[next_stepsound]`

Selects the default footsteps sound pack. The value is an index into the `[stepsoundpack]` array. In all samples, `0` selects the `Normal` pack.

> ⚠️ **Open Question:** Why is this keyword named `next_stepsound` rather than just `stepsound`? The `next_` prefix appears on both metadata keywords. It may relate to the passenger's "next step" behavior, or it may be a legacy naming artifact. **Needs verification.**

#### `[next_roomheight]`

All three samples declare `2`.

> ⚠️ **Open Question:** The value is an integer, not a float. This could mean:
> - `2` meters (interior ceiling height),
> - a reference to a script variable named `2`,
> - a count of interior heights specified elsewhere.
>
> **Hypothesis:** Interior ceiling height in meters. This would be used for rendering passenger headroom or as a fallback bound for the passenger model. **Needs verification.**

---

## 🧩 Chain Assembly

Because pathpoint indices are **per-segment**, assembling a multi-segment vehicle's passenger graph is a non-trivial operation performed by `opts-sim`, not `opts-core`.

### Cross-Segment Link Resolution

The `passengercabin_cfg` files declare cross-segment links via `[linkToPrevVeh]` and `[linkToNextVeh]`. Based on the sample corpus, the interpretation is:

| File | Declared | Value | Physical Meaning |
| :--- | :--- | :--- | :--- |
| GN92_main | `[linkToPrevVeh]` | `16` | Main's pathpoint 16 (at y = −4.331, the physical coupling) |
| GN92_trail | `[linkToNextVeh]` | `0` | Trailer's pathpoint 0 (at y = 3.784, just inside the joint) |

The two pathpoints are at the **same physical location** in the assembled vehicle. The cross-link in the combined graph is therefore:

```
main.pathpoint[16]  ←→  trailer.pathpoint[0]
```

The **naming is non-intuitive**: the main segment (physically in front) declares `[linkToPrevVeh]`, while the trailer (physically behind) declares `[linkToNextVeh]`. This reflects OMSI's internal load order, in which the trailer may be created or indexed before the main vehicle. The keyword names should be read as "link to the previous vehicle *in OMSI's internal array*" and "link to the next vehicle *in OMSI's internal array*", not as spatial directions.

> ⚠️ **Implementation Risk:** This interpretation is consistent with all observed evidence but has not been formally validated against OMSI's runtime behavior. Recommended approach:
>
> 1. Implement the parser to store both links as `Option<PathpointIndex>` without interpretation.
> 2. In `opts-sim`, validate the graph topology by walking from an entry in the main segment to an exit in the trailer.
> 3. If the walk fails, invert the interpretation.
>
> **Do not bake the interpretation into `opts-core`.** Keep it in `opts-sim`, where it can be tested and reversed.

### Combined Graph for a 2-Segment Vehicle

```
MAIN SEGMENT (0..16)                    TRAILER SEGMENT (0..10)
                                        
  ... ── 15 ── 16 ═══════════════════════ 0 ── 1 ── 2 ─┬─ 3 ── 4
                  ↑                     ↑              └─ 5 ─┬─ 6 ── 7
                  │                     │                    └─ 8 ── 9 ── 10
                  │                     │
             main.linkToPrevVeh    trailer.linkToNextVeh
                     16                     0
                     
                  ═══ : cross-segment edge (added by opts-sim)
```

### Algorithm Sketch

```
fn assemble_chain_graph(segments: Vec<(ParsedPaths, ParsedPassengerCabin)>) -> ChainPathGraph:
    let mut graph = ChainPathGraph::new()

    // 1. Add all pathpoints with composite keys
    for (seg_idx, (paths, _)) in segments.iter().enumerate():
        for (pt_idx, pt) in paths.pathpoints.iter().enumerate():
            graph.add_node((seg_idx, pt_idx), pt.position)

    // 2. Add all within-segment edges
    for (seg_idx, (paths, _)) in segments.iter().enumerate():
        for link in paths.pathlinks.iter():
            graph.add_edge((seg_idx, link.from), (seg_idx, link.to))

    // 3. Add cross-segment edges from link keywords in passengercabin
    for (seg_idx, (_, cabin)) in segments.iter().enumerate():
        if let Some(idx) = cabin.link_to_prev_veh:
            // Connect this segment's idx to the previous segment's link_to_next_veh
            if seg_idx > 0:
                let prev_idx = segments[seg_idx - 1].1.link_to_next_veh
                    .expect("previous segment missing link_to_next_veh")
                graph.add_cross_edge((seg_idx - 1, prev_idx), (seg_idx, idx))
        if let Some(idx) = cabin.link_to_next_veh:
            // Symmetric: connect to the next segment's link_to_prev_veh
            if seg_idx + 1 < segments.len():
                let next_idx = segments[seg_idx + 1].1.link_to_prev_veh
                    .expect("next segment missing link_to_prev_veh")
                graph.add_cross_edge((seg_idx, idx), (seg_idx + 1, next_idx))

    return graph
```

### Recommended Type

```rust
pub struct ChainPathGraph {
    /// Nodes keyed by (segment_index, pathpoint_index).
    pub nodes: HashMap<(usize, usize), Pathpoint>,
    /// Directed edges within segments.
    pub edges: Vec<((usize, usize), (usize, usize))>,
    /// Cross-segment edges added from [linkToPrevVeh] / [linkToNextVeh].
    pub cross_links: Vec<((usize, usize), (usize, usize))>,
}
```

---

## 🧪 Minimal Example

A minimal but valid paths file with three pathpoints and two links:

```
[stepsoundpack]
1
Step_01.wav

[pathpnt]
0.0
0.0
0.0

[pathpnt]
1.0
0.0
0.0

[pathpnt]
2.0
0.0
0.0

[next_stepsound]
0

[next_roomheight]
2

[pathlink]
0
1

[pathlink]
1
2
```

---

## ✅ Resolved Questions

| ID | Question | Resolution |
| :--- | :--- | :--- |
| Q-001 | Are pathlinks directed or undirected? | `[pathlink]` is bidirectional. One-way directed links use `[pathlink_oneway]` (📚 M-0006). |
| Q-002 | Are pathpoint coordinates relative to the segment's local origin? | Yes — segment-local coordinate space relative to the vehicle/segment origin (confirmed by M-0020). |
| Q-003 | Meaning of `[next_roomheight]`'s value. | Ceiling height in metres for subsequent links to prevent passenger head clipping (📚 M-0006). |
| Q-005 | How sound packs are selected at runtime. | `[next_stepsound]` assigns the zero-based footstep sound index from `[stepsoundpack]` to subsequent links (📚 M-0006). |
| Q-006 | Are pathpoint indices global across all vehicles in a chain, or per-segment? | **Per-segment.** Every segment's paths file starts at index 0. Confirmed by the trailer sample. |

---

## ❓ Open Questions

| ID | Question | Priority | Status |
| :--- | :--- | :--- | :--- |
| Q-004 | Why the `next_` prefix on metadata keywords? | 🟢 Low | Open (stylistic convention for modal state commands). |
| Q-007 | Are link declarations required to be non-self-referential and non-duplicated? | 🟢 Low | Open (parser guards against self-loops and duplicates). |

---

## 🌐 Community Cross-Check (M-0006)

Facts marked 📚 come from the OMSI community wiki (`reboot.omsi-webdisk.de`, CC BY-SA 4.0; facts restated, nothing copied) (passenger-paths page); ✅ = confirmed in the corpus.

- **`[pathlink_oneway]` — missing from this spec.** ✅ 887 uses in 107 of 455 paths files. 📚 It connects two pathpoints **in one direction only** (from the first-named point to the second), used to stop passengers using an exit as an entrance. `[pathlink]` is bidirectional; a missing link makes passengers "teleport". OPTS reads it as a directed link (2026-10-21): the NF6D's entrances reach the cabin only through such links, and with them ignored nobody could walk in.
- **Group commands** (📚): `[next_roomheight]` sets the ceiling height (m) for the following links (avoids head clipping); `[next_stepsound]` assigns a zero-based footstep sound index to following links; `[stepsoundpack]` (count + files) must precede the first link ✅ (446 uses).
- **Construction rules** (📚): pathpoints lie on the floor (1–2 cm above, never below); keep ≥ 25–30 cm between points; entry/exit points at the vehicle edge, ≥ 30–40 cm from the door threshold; avoid four-way junctions; ≤ 45° between consecutive points; keep seats close to the path to limit visible "popping".
- Limits (📚): standing→sitting is instantaneous (no transition animation); occupancy is limited to defined positions.

---

## 📝 Provenance

- **Community cross-check (M-0006, 2026-10-04):** wiki page `16-fahrgastwege`; corpus keyword census over 455 paths files (found undocumented `[pathlink_oneway]`).
- **Sample `.cfg` files (2024-MM-DD):** Three files provided by the OPTS project lead: `paths_EN92.cfg`, `paths_GN92_main.cfg`, and `paths_GN92_trail.cfg`. Observed as-is.
- **Embedded German comments (2024-MM-DD):** Pathpoint name comments (e.g., `Tür VV`, `Kasse`, `Entwerter`, `Übergang`) were translated from German. These are documentation written by the original modders.
- **Encoding:** Windows-1252, confirmed again via mojibake observations (`Tür` rendered as both `T�r` and `T黵` under different display conditions).
- **Cross-file evidence (2024-MM-DD):** The `[coupling_back]` and `[coupling_front]` coordinates in `MAN_GN92_main.bus` and `MAN_GN92_trail.bus` were compared to the pathpoint coordinates at the articulation boundary. The main segment's pathpoint 16 y-coordinate matches the coupling exactly.
- **Project lead clarification (2024-MM-DD):** Pathpoint indices are per-segment, restarting at 0 for every vehicle segment.

---

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.bus` — Vehicle Definition Files](bus.md)
- [`model_cfg` — Model Configuration Files](model_cfg.md)
- [`passengercabin_cfg` — Passenger Cabin Configuration Files](passengercabin_cfg.md)
- [Text Formats Index](../README.md)
- Simulation: Passenger Model *(planned)* *(not yet written)*
- Simulation: Transit Network *(planned)* *(not yet written)*
- Architecture: Chain Assembly *(planned)* *(not yet written)*
- [ADR-0002: Clean-Room Policy](../../../adr/0002-clean-room-policy.md)
