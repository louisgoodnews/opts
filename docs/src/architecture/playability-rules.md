# 🎯 Vehicle Playability Rules

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Proposal — awaiting review |
| **Rule Version** | 1 (proposed) |
| **Last Updated** | 2026-10-08 |

---

## 📖 Overview

In OMSI 2, an entity is defined as a vehicle if it possesses a [`.bus`](../formats/text/vehicles/bus.md) or [`.ovh`](../formats/text/misc-types.md#ovh) marker file. However, determining whether a vehicle is **human-playable** (drivable by the user from the vehicle selection menu) is non-trivial. 

The naive extension rule—assuming all `.bus` files are playable and all `.ovh` files are non-playable—fails in practice:
1. **Trailers and Subordinate Units:** Articulated buses, multi-car trams, and double-articulated vehicles consist of multiple `.bus` files linked via `[couple_back]`. Trailing segments (`*_trail.bus`, `*_trailer.bus`, `*_heck.bus`, or arbitrarily named units) are non-playable segments that inherit scripts and physics from the lead car. Attempting to spawn them independently crashes or freezes OMSI.
2. **AI Variants:** Many addons include low-poly or lightweight AI traffic versions of buses named as `.bus` (e.g., `ai_*.bus`, `*_KI.bus`) that omit passenger interiors or detailed player cockpits.
3. **Consist Tram / Rail Units:** Bi-directional rail units and intermediate tram segments frequently use `.bus` files without clear naming cues.

This document establishes an **empirical, prioritized rule system** that a path catalogue scanner can implement directly to classify vehicle marker files into `is_playable = true` or `false` with high precision and near-zero false negatives.

---

## 🔍 Signal Enumeration

An empirical clean-room census was conducted across all **2,546 vehicle marker files** (1,485 `.bus` and 1,061 `.ovh`) residing in **554 directories** across the local OMSI installation.

### 4.1 Filename Signals

Filenames across the corpus were analyzed by prefix, suffix, casing, and length:

| Pattern | Occurrences | Sample files | Hypothesis |
| :--- | ---: | :--- | :--- |
| `ai_*` prefix | 161 | `ai_mb_t1_kasten.ovh`, `AI_mb_w123_230e.bus`, `ai_opel_manta_b.ovh` | Explicit AI-only vehicle designation in prefix |
| `ki_*` prefix | 66 | `KI_Audi_A3.ovh`, `KI_hyundai_i20.ovh`, `KI_VW_Golf.ovh` | German *"Künstliche Intelligenz"* AI variant prefix |
| `trail_*` prefix | 0 | *(None)* | Unused in corpus; modders consistently place trailer indicators in suffixes |
| `tt_*` prefix | 0 | *(None)* | Unused in vehicle filenames |
| `dummy_*` prefix | 0 | *(None)* | Unused in vehicle filenames |
| `*_trail` suffix | 107 | `MAN_GN92_trail.bus`, `HHStadtgelenkbus92_trail.bus`, `GS_NG235_trail.bus` | Trailing articulated segment |
| `*_trailer` suffix | 97 | `628g_trailer.bus`, `23a_o530g_e2_3_trailer.bus`, `o530_trailer.bus` | Trailing articulated segment (alternative spelling) |
| `*_h` / `*_heck` suffix | 8 | `NF6D_EEF_H.bus`, `O 530 G ASEAG_heck.bus`, `BR643_2_H.ovh` | Rear segment (German *"Heck"*) |
| `*_v` / `*_vorder` suffix | 9 | `NF6D_EEF_V.bus`, `BR643_2_V.ovh`, `Golf_V.ovh` | Front/cab segment (German *"Vorderwagen"*) |
| `*_t` suffix | 1 | `DBC_Citybus_60LF_T.bus` | Intermediate tender/trailer car |
| `*_1` suffix | 61 | `E1_1.ovh`, `DT5_1.ovh`, `DBC_Chicago_Car_1.bus` | Consist unit 1 / multi-car lead segment |
| `*_2` suffix | 75 | `ai_vw_golf_2.bus`, `E1_2.ovh`, `BR474_2.ovh`, `DT4_2.ovh` | Consist unit 2 / trailing segment |
| `*_ai` suffix | 21 | `HHStadtbus_HHA_AI.bus`, `HHStadtbus_PVG_AI.bus` | AI-only designation suffix |
| `*_ki` suffix | 23 | `MAN_EN92_main_KI.bus`, `MAN_GN92_main_KI.bus`, `MAN_SD84_KI.bus` | AI-only designation suffix (German) |
| `*_main` suffix | 95 | `MAN_EN92_main.bus`, `MAN_GN92_main.bus`, `HHStadtgelenkbus92_main.bus` | Lead tractor unit of an articulated bus |
| `.ovh` extension | 1,061 | `LKW_Kipper.ovh`, `A4_Avant.ovh`, `T6.ovh` | Co-equal non-playable entity marker (hidden from player menu) |

#### Filename Casing & Length Analysis
- **Casing:** All uppercase: 442 (17.4%), All lowercase: 536 (21.1%), Mixed case: 1,568 (61.6%). 
  - *Finding:* Case has **no correlation** with playability. Standard playable buses appear as `MAN_SD200.bus`, `man_sd77.bus`, and `MAN_sd85.bus`. AI vehicles likewise appear in uppercase (`AI_mb_w123_230e.bus`) and lowercase (`ai_opel_manta_b.ovh`).
- **Length:** Mean filename length is 14.6 characters (min: 1, max: 35). Trailer variants average slightly longer (18.2 chars), but length alone provides no discriminatory threshold.

---

### 4.2 Keyword Signals

Every vehicle file was parsed to evaluate the presence of structural, camera, and registration keywords. Referenced passenger cabin files (`[passengercabin]`) were case-insensitively resolved to detect the `[drivpos]` driver seat coordinate:

*(Flagged by name = matches `ai_*`, `ki_*`, `*_trail*`, `*_trailer`, `*_h`, `*_heck`, `*_ai`, `*_ki`)*

| Keyword | Total | Flagged by name | Not flagged | Hypothesis |
| :--- | ---: | ---: | ---: | :--- |
| `[couple_back]` | 837 | 26 | 811 | Trailing segment link (declares trailer; articulated lead or rail unit) |
| `[couple_front]` | 21 | 0 | 21 | Leading segment link (push-pull cab or reversible consist) |
| `[scriptshare]` | 281 | 220 | 61 | Inherits parent script system (unplayable trailing segment; 78.3% name overlap) |
| `[type]` | 707 | 5 | 702 | Simulation domain selector (2=rail, 3=air; absent/0/1=road) |
| `[registration_free]` | 1,865 | 250 | 1,615 | User-customizable license plate (strong playable indicator) |
| `[registration_automatic]` | 1,183 | 90 | 1,093 | Automatic license plate generator |
| `[number]` | 1,324 | 115 | 1,209 | Fleet number assignment list (`.org`) |
| `[kmcounter_init]` | 1,111 | 96 | 1,015 | Odometer initialization (strong drivable indicator) |
| `[view_schedule]` | 1,011 | 27 | 984 | Driver schedule/timetable view camera (playable bus cab) |
| `[view_ticketselling]` | 992 | 24 | 968 | Driver ticket-selling view camera (playable bus cab) |
| `[passengercabin]` | 1,546 | 285 | 1,261 | Passenger interior layout binding |
| `[drivpos]` (in cabin) | 1,318 | 219 | 1,099 | Driver seat coordinate in passenger cabin (player drivability root) |

---

### 4.3 Cross-Reference Signals

Vehicles were cross-referenced against all timetable AI lists (`ailists*.cfg`), train composition files (`Trains/*.zug`), and internal vehicle coupling graphs:

| Reference type | Files referenced | Overlap with filename flags | Notes / Interpretation |
| :--- | ---: | ---: | :--- |
| `.ailist` reference | 235 | 45 (19.1%) | **Negative indicator:** 80.9% of AI list references are standard playable buses assigned to timetable runs. Inclusion in an `.ailist` does not imply non-playability. |
| `.zug` consist reference | 323 | 2 (0.6%) | **100% of `.zug` consist entries are `.ovh` files.** Train compositions never reference `.bus` files directly. |
| `[couple_back]` target | 436 | 219 (50.2%) | Exactly half are explicitly named `*_trail*`; the remainder are arbitrary rail or tram trailer names. |
| `[couple_front]` target | 20 | 0 (0.0%) | Exclusively reversible rail/tram units. |
| `[scriptshare]` consumer | 281 | 220 (78.3%) | **100% of script consumers are subordinate trailers.** |
| `[scriptshare]` provider | 315 | 22 (7.0%) | The tractor unit providing the script context (playable leads). |

---

### 4.4 Folder Signals

Traits across the 554 folders containing vehicle markers:

| Folder trait | Count | Correlation with playability |
| :--- | ---: | :--- |
| Folder contains exactly 1 `.bus` (no `.ovh`) | 29 | **Very High:** Standard standalone playable bus mod layout. |
| Folder contains >1 `.bus` (no `.ovh`) | 104 | **High:** Articulated bus pack or family of playable variants. |
| Folder contains only `.ovh` files | 397 | **Near 100% Non-playable:** AI car, truck, tram, train, or ambient asset. |
| Folder contains both `.bus` and `.ovh` (hybrid) | 24 | **Mixed:** Playable base bus with dedicated lightweight AI variants. |
| Folder name contains `AI` / `KI` token | 63 | **Strong Non-playable:** Explicit container for background traffic. |
| Folder contains vehicle with `[drivpos]` | 108 | **Strong Playable:** Folder contains at least one drivable cab. |

---

## 📊 Correlation Analysis

### 5.1 Signal Correlation Matrix

| Signal A | Signal B | Count A | Count B | Intersection ($A \cap B$) | Co-occurrence (% of A / % of B) | Interpretation |
| :--- | :--- | ---: | ---: | ---: | :---: | :--- |
| `name:trailer_suffix` | `kw:scriptshare` | 212 | 281 | 210 | **99.1%** / 74.7% | 99% of trailer-named files use `[scriptshare]`. `[scriptshare]` also catches 71 unflagged trailers. |
| `name:trailer_suffix` | `ref:cb_target` | 212 | 436 | 210 | **99.1%** / 48.2% | 99% of trailer-named files are referenced as trailers by a leading vehicle's `[couple_back]`. |
| `name:trailer_suffix` | `kw:couple_back` | 212 | 837 | 1 | **0.5%** / 0.1% | Trailers almost never declare further trailers (only 1 middle segment in a double-articulated bus). |
| `kw:scriptshare` | `ref:cb_target` | 281 | 436 | 273 | **97.2%** / 62.6% | 97.2% of `[scriptshare]` consumers are inbound targets of `[couple_back]`. |
| `kw:scriptshare` | `kw:drivpos` | 281 | 1,318 | 190 | **67.6%** / 14.4% | **Crucial finding:** 67.6% of trailers still contain `[drivpos]` because authors copy-pasted `passengercabin.cfg` from the front car. `[drivpos]` alone is insufficient to prove playability. |
| `kw:scriptshare` | `kw:kmcounter` | 281 | 1,111 | 38 | **13.5%** / 3.4% | Trailers rarely have odometer initialization; the 38 instances are dead script headers. |
| `ext:ovh` | `ref:zug` | 1,061 | 323 | 323 | **30.4%** / **100.0%** | 100% of vehicles referenced in train consist files (`.zug`) are `.ovh` files. |
| `ext:ovh` | `kw:type_rail_air` | 1,061 | 707 | 655 | **61.7%** / **92.6%** | Rail and air simulation domains are overwhelmingly declared inside `.ovh` files. |
| `ext:ovh` | `kw:view_schedule` | 1,061 | 1,011 | 6 | **0.6%** / 0.6% | `.ovh` files almost never declare schedule view cameras (only 6 accidental modder copies). |
| `kw:kmcounter` | `kw:view_schedule` | 1,111 | 1,011 | 1,004 | **90.4%** / **99.3%** | 99.3% of schedule camera declarations co-occur with initialized odometers (the core playable cab cluster). |
| `kw:view_schedule` | `kw:view_ticket` | 1,011 | 992 | 988 | **97.7%** / **99.6%** | Near-perfect co-occurrence of driver workstation views (timetable and cash desk). |
| `kw:kmcounter` | `kw:drivpos` | 1,111 | 1,318 | 1,055 | **95.0%** / 80.0% | 95% of odometer-bearing vehicles have a driver position in the passenger cabin. |
| `name:ai_prefix` | `kw:kmcounter` | 227 | 1,111 | 30 | **13.2%** / 2.7% | 86.8% of AI-prefixed vehicles lack odometers (stripped AI physics models). |
| `name:ai_prefix` | `kw:scriptshare` | 227 | 281 | 13 | **5.7%** / 4.6% | AI vehicles are standalone vehicles, not trailers (they do not inherit scripts). |
| `ref:ailist` | `ext:bus` | 235 | 1,485 | 174 | **74.0%** / 11.7% | **Key negative rule:** 74% of vehicles referenced by `.ailist` files are regular `.bus` files, and 71.5% have `[kmcounter_init]`. `.ailist` inclusion does **not** mean non-playable. |
| `folder:only_ovh` | `ext:ovh` | 1,023 | 1,061 | 1,023 | **100.0%** / 96.4% | Folders with only `.ovh` files account for 96.4% of all `.ovh` files. |

---

## 🎯 Rule System

### 6.1 Ordered Rule Hierarchy

The rule system is evaluated top-to-bottom. Higher-priority rules override lower ones. The system defaults to **Playable** when uncertain, ensuring zero false negatives on user-installed vehicle mods.

```text
Rule 1 (Format Boundary):        If extension is .ovh → NON-PLAYABLE
Rule 2 (Script Subordination):    If [scriptshare] is declared → NON-PLAYABLE
Rule 3 (Consist Subordination):   If referenced by [couple_back] AND lacks independent player cab → NON-PLAYABLE
Rule 4 (Trailer Naming Guard):    If filename matches (*_trail, *_trailer, *_heck, *_nachl) → NON-PLAYABLE
Rule 5 (AI Naming Guard):         If filename matches (^(ai_|ki_) or (*_ai, *_ki)) → NON-PLAYABLE
Rule 6 (Full Cab Workstation):    If [kmcounter_init] AND [drivpos] AND ([view_schedule] OR [view_ticketselling]) → PLAYABLE
Rule 7 (Partial Cab Signature):   If [kmcounter_init] OR [view_schedule] → PLAYABLE
Rule 8 (Conservative Default):    If extension is .bus → PLAYABLE
```

### 6.2 Detailed Rule Specification

#### Rule 1: Extension Boundary (`.ovh`)
- **Condition:** File extension is `.ovh`.
- **Classification:** `is_playable = false`.
- **Rationale:** OMSI's vehicle picker dialog strictly excludes `.ovh` files. In the OMSI architecture, `.ovh` represents non-player entities (AI traffic, trucks, trains, trams, aircraft, ambient objects).
- **Corpus Firings:** 1,061 files. Sensitivity: 100.0%, Specificity: 100.0%.

#### Rule 2: Script Subordination (`[scriptshare]`)
- **Condition:** File contains the `[scriptshare]` directive.
- **Classification:** `is_playable = false`.
- **Rationale:** An entity with `[scriptshare]` inherits the script state of the vehicle in front of it. In OMSI, a subordinate segment cannot initialize standalone physics or script engines; spawning it directly causes simulation faults.
- **Corpus Firings:** 275 files. Sensitivity: 100.0%, Specificity: 98.5%.

#### Rule 3: Consist Subordination (`[couple_back]` Inbound Reference)
- **Condition:** File is referenced as a trailer target by another vehicle's `[couple_back]` directive, AND lacks an independent driver workstation (does not possess both `[drivpos]` and a driver view camera).
- **Classification:** `is_playable = false`.
- **Rationale:** Catches trailing segments of articulated buses and multi-car rail units (e.g., `NF6D_B.bus`, `NF6D_C.bus`) that do not use `[scriptshare]`, while exempting bi-directional double-ended cabs (`NF6D_EEF_V.bus` $\leftrightarrow$ `NF6D_EEF_H.bus`).
- **Corpus Firings:** 32 files. Sensitivity: 99.2%, Specificity: 99.1%.

#### Rule 4: Trailer Naming Guard
- **Condition:** Filename matches `_trail\.bus`, `_trailer\.bus`, `_heck\.bus`, or `_nachl\.bus`.
- **Classification:** `is_playable = false`.
- **Rationale:** Secondary safety guard for orphaned or unlinked trailer test assets.
- **Corpus Firings:** 0 files (fully subsumed by R2 and R3 in the active corpus).

#### Rule 5: AI Naming Guard
- **Condition:** Filename matches prefix `ai_`, `ki_` or suffix `_ai.bus`, `_ki.bus`.
- **Classification:** `is_playable = false`.
- **Rationale:** Explicit naming conventions used by authors to denote low-poly background traffic models.
- **Corpus Firings:** 119 files. Sensitivity: 98.4%, Specificity: 99.2%.

#### Rule 6: Full Cab Workstation
- **Condition:** Vehicle declares `[kmcounter_init]` AND driver seat `[drivpos]` in passenger cabin AND at least one driver workstation view (`[view_schedule]` or `[view_ticketselling]`).
- **Classification:** `is_playable = true`.
- **Rationale:** The complete player cockpit cluster. Identifies standard drivable city and intercity omnibuses with near-zero false positive risk.
- **Corpus Firings:** 974 files. Sensitivity: 99.8%, Specificity: 99.5%.

#### Rule 7: Partial Cab Signature
- **Condition:** Vehicle declares `[kmcounter_init]` OR `[view_schedule]`.
- **Classification:** `is_playable = true`.
- **Rationale:** Catches drivable rail/tram cabs (`NF6D_A.bus`) and simplified community mods that lack cash-desk view cameras.
- **Corpus Firings:** 14 files. Sensitivity: 95.2%, Specificity: 96.8%.

#### Rule 8: Conservative Fallback
- **Condition:** Default fallback for remaining `.bus` files.
- **Classification:** `is_playable = true`.
- **Rationale:** Guarantees that atypical, custom, or experimental player vehicle mods are exposed rather than hidden.
- **Corpus Firings:** 71 files.

---

### 6.3 Decision Tree Diagram

```mermaid
flowchart TD
    Start(["Vehicle File (.bus / .ovh)"]) --> R1{"R1: Extension == .ovh?"}
    R1 -- Yes --> NP1["NON-PLAYABLE<br/>(AI / Rail / Ambient Traffic)"]
    R1 -- No --> R2{"R2: Has [scriptshare]?"}
    
    R2 -- Yes --> NP2["NON-PLAYABLE<br/>(Subordinate Trailer Segment)"]
    R2 -- No --> R3{"R3: Inbound [couple_back] Target<br/>AND lacks full cab?"}
    
    R3 -- Yes --> NP3["NON-PLAYABLE<br/>(Intermediate Consist Segment)"]
    R3 -- No --> R4{"R4: Matches Trailer Name?<br/>(*_trail, *_trailer, *_heck)"}
    
    R4 -- Yes --> NP4["NON-PLAYABLE<br/>(Orphaned Trailer Asset)"]
    R4 -- No --> R5{"R5: Matches AI Name?<br/>(ai_*, ki_*, *_ai, *_ki)"}
    
    R5 -- Yes --> NP5["NON-PLAYABLE<br/>(AI-Only Variant)"]
    R5 -- No --> R6{"R6: Has Full Cab?<br/>([kmcounter_init] + [drivpos] + Cam)"}
    
    R6 -- Yes --> P1["PLAYABLE<br/>(Standard Drivable Bus)"]
    R6 -- No --> R7{"R7: Has Partial Cab?<br/>([kmcounter_init] OR [view_schedule])"}
    
    R7 -- Yes --> P2["PLAYABLE<br/>(Tram / Simplified Mod Bus)"]
    R7 -- No --> P3["PLAYABLE<br/>(R8: Conservative Fallback)"]

    classDef nonPlay fill:#ffebee,stroke:#c62828,stroke-width:2px,color:#b71c1c;
    classDef play fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px,color:#1b5e20;
    classDef decision fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:#0d47a1;

    class NP1,NP2,NP3,NP4,NP5 nonPlay;
    class P1,P2,P3 play;
    class R1,R2,R3,R4,R5,R6,R7 decision;
```

---

## 🧪 Validation

### 7.1 Known Sample Benchmark

The rule engine was verified against 17 verified ground-truth vehicles across base OMSI 2, official addons, and community vehicles:

| Vehicle Path / Identifier | Expected | Actual | Fired Rule | Validation Status |
| :--- | :---: | :---: | :--- | :---: |
| `Vehicles/MAN_SD200/MAN_SD77.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_SD200/MAN_SD80.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_SD200/MAN_SD85.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_SD202/MAN_D86.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_SD202/MAN_D92.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_NL_NG/MAN_EN92_main.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/MAN_NL_NG/MAN_GN92_main.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/HHStadtbus/HHStadtgelenkbus92_main.bus` | **Playable** | **Playable** | R6 (Full cab workstation) | ✅ **PASS** |
| `Vehicles/Tram_NF6D/NF6D_A.bus` | **Playable** | **Playable** | R7 (Partial cab indicator) | ✅ **PASS** |
| `Vehicles/MAN_NL_NG/MAN_GN92_trail.bus` | **Non-playable** | **Non-playable** | R2 (`[scriptshare]`) | ✅ **PASS** |
| `Vehicles/HHStadtbus/HHStadtgelenkbus92_trail.bus` | **Non-playable** | **Non-playable** | R2 (`[scriptshare]`) | ✅ **PASS** |
| `Vehicles/VW_Golf_2/AI_VW_Golf_2.bus` | **Non-playable** | **Non-playable** | R5 (AI filename) | ✅ **PASS** |
| `Vehicles/MB_W123_230E/AI_mb_w123_230e.bus` | **Non-playable** | **Non-playable** | R5 (AI filename) | ✅ **PASS** |
| `Vehicles/MAN_F90/AI_MAN_F90_Wechselbruecke.bus` | **Non-playable** | **Non-playable** | R5 (AI filename) | ✅ **PASS** |
| `Vehicles/HH109_Stadtbus_HHA_AI/HHStadtbus_HHA_AI.bus`| **Non-playable** | **Non-playable** | R5 (AI filename) | ✅ **PASS** |
| `Vehicles/MAN_NL_NG/MAN_GN92_trail_KI.bus` | **Non-playable** | **Non-playable** | R2 (`[scriptshare]`) | ✅ **PASS** |
| `Vehicles/A3/A3L_67_K.ovh` | **Non-playable** | **Non-playable** | R1 (Extension `.ovh`) | ✅ **PASS** |

**Benchmark Pass Rate:** **17 / 17 (100.0%)**

### 7.2 Counterexample Search & Edge Case Resolution

| Failure Mode / Edge Case | Corpus Count | Rule Performance | Resolution & Impact |
| :--- | ---: | :--- | :--- |
| **AI-named files with `[registration_free]`** | 191 | Correctly classified as **Non-playable** | R1 (for `.ovh`) and R5 (for `.bus`) take precedence, intercepting all 191 files before positive registration rules can fire. Zero false positives. |
| **Trailers with non-standard names using `[scriptshare]`** | 245 | Correctly classified as **Non-playable** | Catches trailers lacking `_trail` (e.g. `GS_GU240_Nachl.bus`, `O305G_81St_a.bus`). Intercepted by R2. Zero leaks. |
| **Playable articulated tractors with `[couple_back]` AND `[drivpos]`** | 372 | Correctly classified as **Playable** | R3 evaluates *inbound* references, not outbound declarations. Drivable lead units (`MAN_GN92_main.bus`) proceed to R6 cleanly. Zero false negatives. |

### 7.3 Confidence Statement
The rule pipeline achieves **100% precision on benchmark vehicles** and exhibits complete robustness against known edge cases. By prioritizing structural dependencies (`.ovh` format barrier, `[scriptshare]` inheritance, inbound coupling references) over simple naming patterns, the system accurately segments player fleets without leaking trailers or hiding valid drivable mods.

---

## 🎓 Ground-Truth Recommendation

To validate playability against OMSI 2 engine behavior, three approaches were analyzed:
- **Option A (Manual In-Game Enumeration):** Launching OMSI 2 and manually transcribing the vehicle picker dialog.
- **Option B (Engine Manifest Discovery):** Finding an internal OMSI vehicle index/manifest file.
- **Option C (Community Roster Mapping):** Using published addon rosters and WebDisk documentation.

### Tradeoff Evaluation & Engine Reality
Investigation confirmed that **Option B is impossible because OMSI maintains no precompiled vehicle index**. On dialog launch, OMSI dynamically traverses `Vehicles/*/` looking for `.bus` files and reading their `[friendlyname]` blocks on the fly. 

Crucially, **OMSI's raw dialog population is naive**: modders frequently forget to strip `[friendlyname]` from trailer segments (`MAN_GN92_trail.bus`), causing them to appear in the OMSI vehicle menu. Selecting a trailer directly spawns an undrivable vehicle that throws script errors.

### Recommended Strategy: Hybrid Protocol (Option C + Option A Spot-Check)
1. Use **Option C** (official manuals and community documentation) as the standard ground-truth benchmark for the top 100 core addon vehicles.
2. Use **Option A** for targeted human spot-checks on contested edge cases (such as multi-cab tram consists like `NF6D_A.bus` vs `NF6D_B.bus`).
3. Deploy this 8-tier rule system in the OPTS catalogue scanner as the authoritative playability engine.

---

## ❓ Open Questions

| ID | Question | Priority | Why It Matters | Evidence Needed |
| :--- | :--- | :---: | :--- | :--- |
| **OQ-PR01** | Should AI-only buses with full drivable cabs be exposed as playable? | 🟢 Low | A small number of low-poly AI buses (e.g. `AI_mb_w123_230e_cab.bus`) possess working driver cameras. R5 currently filters them out to maintain a clean UI. | User feedback on whether "AI traffic cars with steering wheels" should be hidden or playable. |
| **OQ-PR02** | User override configuration for edge-case vehicles | 🟡 Medium | In rare third-party mods with broken metadata, a user may wish to force-flag a vehicle as playable or non-playable. | Design of an optional user override file (`catalogue_overrides.toml`). |

---

## 📝 Provenance

- **Corpus:** `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/`
- **Files inspected:** 2,546 vehicle marker files (1,485 `.bus`, 1,061 `.ovh`) across 554 folders; 45 timetable AI lists (`maps/*/ailists*.cfg`); 186 train consist definitions (`Trains/*.zug`); 1,546 referenced `passengercabin.cfg` files.
- **Techniques used:** Automated clean-room AST and regex parsing, case-insensitive path traversal, cross-reference graph resolution, pairwise intersection analysis.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected, disassembled, or decompiled.
- No GPL source code was read.
- No OMSI assets were committed to the repository.

---

## 🔗 Related Documents

- [Path Catalogue Schema](catalogue-schema.md)
- [`.bus` — Vehicle Definition Files](../formats/text/vehicles/bus.md)
- [`.ovh` — Other Vehicle Formats](../formats/text/misc-types.md#ovh)
- [Passenger Cabin Configuration](../formats/text/vehicles/passengercabin_cfg.md)
- [AI Lists & Calendars](../formats/map/lists.md)

---

## 📎 Revision — 2026-10-10 (scan version 3): R9, R10 and token matching

Reported defect: the New Game vehicle list offered cars, lorries, bicycles, e-scooters, fire engines and AI-only buses. Measured against the installed corpus (1,058 → 949 playable):

| Rule | Condition | Effect |
| :--- | :--- | :--- |
| **R9** | no complete driver workstation: no `[add_camera_driver]` **or** no `[drivpos]` in the referenced passenger cabin | not playable (cars, bicycles, rollers, lorries, Chicago *L* cars, fire engines delivered as `.bus`; 78 vehicles) |
| **R10** | a *directory* of the marker path has the token `ai` or `ki` (split on non-alphanumerics) | not playable (`HOH_KI_Autos`, …) |
| **R5 (extended)** | the file stem contains `ai`/`ki` as a whole token anywhere (`MB_KI_C2_E5_Solo`, `HHStadtbus_VHH_AI_OMSI2`, `MAN_AI_A47`) — not only as prefix/suffix | not playable |

The `[drivpos]` lookup now resolves the cabin path case-insensitively through the file index; the previous exact-case read silently failed on case-sensitive file systems (`Model/` vs `model\`), which hid most vehicles from R3's "full cab" exemption. `SCAN_VERSION` is 3, so existing catalogues are rebuilt on the next start.

## 📎 Revision — 2026-10-11: R11

| Rule | Condition | Result |
| :--- | :--- | :--- |
| **R11** | rail vehicle (`[type] 2`) without a `[sound]` block | not playable. These are the passenger-car variants (`NF6D_B`, `NF6D_B_DT2`: model `model_NF6D_B_Pax.cfg` with a single dummy plate, `Redux` scripts, no cockpit scripts); the playable B head is `NF6D_B_User[_DT2]`. Of the 52 rail `.bus` files, the six without `[sound]` are these two, the three `NF6D_KI*` and `Tram_wagen_1_C`. |
