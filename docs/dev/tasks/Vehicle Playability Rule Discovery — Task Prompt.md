# 🎯 Vehicle Playability Rule Discovery

You are deriving a rule system to classify OMSI vehicle markers as
**playable** or **non-playable**. This is a pattern-discovery task
across the entire `Vehicles/` tree.

The extension-based rule (`.bus` = playable, `.ovh` = non-playable)
is insufficient. `.bus` files include AI variants, trailer segments,
and other non-playable content. Filenames suggest this
(`ai_*.bus`, `*_trail.bus`, `*_H.bus`), but the pattern must be
characterized, quantified, and validated.

The output is a rule system document that a scanner can implement
directly.

This is a RESEARCH task. You are not writing Rust code. You are
gathering evidence and proposing rules.

---

## 1. Configuration

### OMSI installation path
/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2

If this path does not exist or is inaccessible, STOP and report.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Output target
Create `docs/src/architecture/playability-rules.md` (new file).

If the file already exists, STOP and report. Do not overwrite.

### Related context (read first)
- `docs/src/architecture/catalogue-schema.md` (the current schema
  design and prior investigations)
- `docs/src/formats/text/vehicles/bus.md`
- `docs/src/formats/text/misc-types.md` (`.ovh` section)
- `docs/src/formats/text/vehicles/passengercabin_cfg.md`
- `docs/src/formats/map/lists.md` (AI list format)

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not sample any file with extension `.exe`, `.dll`, `.so`,
  `.pdb`, `.lib`, or `.obj`.
- Do not modify any file in the OMSI installation.
- Do not copy any OMSI file into the repository.
- Do not quote more than 20 consecutive lines from any OMSI file.
- Do not write Rust code.
- Do not modify the schema proposal document. This task appends a
  new document; the schema proposal may be updated later.

### ✅ Permitted

- Walk `Vehicles/` recursively.
- Read all `.bus` and `.ovh` files in full.
- Read `.ailist`, `.zug`, `.cti`, and referenced `.cfg` files.
- Read `passengercabin.cfg` files to check for `[drivpos]`.
- Cross-reference existing specs.
- Produce a new playability rule document.

---

## 3. Preconditions

Verify and report before starting:

1. `docs/src/architecture/catalogue-schema.md` exists with Stage 2
   findings.
2. The Stage 2 finding "`.bus` and `.ovh` are co-equal vehicle
   markers" is present.
3. The 550 vehicle folders figure is confirmed (or corrected).
4. `docs/src/formats/text/vehicles/bus.md` documents `[couple_back]`,
   `[couple_front]`, `[scriptshare]`, and `[type]`.

If any precondition fails, STOP and report.

---

## 4. Signal Enumeration

Enumerate every signal that could distinguish playable from
non-playable vehicles. Signals fall into these categories:

### 4.1 Filename Signals

Walk all `.bus` and `.ovh` files. For each, record the filename.
Then group by pattern:

- **Prefix:** `ai_`, `AI_`, `ki_`, `KI_`, `trail_`, `_trail`,
  `tt_`, `dummy_`, etc.
- **Suffix:** `_trail`, `_Trailer`, `_H`, `_V`, `_T`, `_1`, `_2`,
  `_AI`, `_ai`, etc.
- **Case:** does filename case correlate with playability?
- **Length:** does filename length differ?

For each pattern, count occurrences and list 5 samples.

**Deliverable:** A subsection titled "Filename Signals" with a
table:

| Pattern | Occurrences | Sample files | Hypothesis |
| :--- | ---: | :--- | :--- |

### 4.2 Keyword Signals

For each `.bus`/`.ovh` file, extract the presence (or value) of
these keywords:

- `[couple_back]` — trailer references
- `[couple_front]` — tractor references
- `[scriptshare]` — inherited scripts
- `[type]` — vehicle class
- `[registration_free]` — free registration (suggests playable)
- `[registration_automatic]` — automatic registration
- `[number]` — fleet number list
- `[kmcounter_init]` — odometer init (drivable)
- `[view_schedule]` — driver schedule view
- `[view_ticketselling]` — driver ticket view
- `[passengercabin]` — passenger cabin reference
- `[drivpos]` in the referenced passengercabin file

For each keyword, count:
- Total files with the keyword
- Files with the keyword that are also flagged by a filename
  signal (`ai_*`, `*_trail`, etc.)
- Files with the keyword that are NOT flagged by any filename
  signal

**Deliverable:** A subsection titled "Keyword Signals" with a table:

| Keyword | Total | Flagged by name | Not flagged | Hypothesis |
| :--- | ---: | ---: | ---: | :--- |

### 4.3 Cross-Reference Signals

For each `.bus`/`.ovh` file, determine:

- Is it referenced by any `.ailist` file? (AI usage)
- Is it referenced by any `.zug` file? (Consist usage)
- Is it referenced by any other `.bus`/`.ovh` file via
  `[couple_back]`? (Trailer)
- Is it referenced by any other `.bus`/`.ovh` file via
  `[couple_front]`? (Tractor)
- Is it referenced by any `.bus`/`.ovh` file via `[scriptshare]`?
  (Shared script consumer)
- Is it referenced by any `.bus`/`.ovh` file via `[scriptshare]`
  as a script *source*? (Shared script provider)

**Deliverable:** A subsection titled "Cross-Reference Signals" with
a table:

| Reference type | Files referenced | Overlap with filename flags |
| :--- | ---: | :--- |

### 4.4 Folder Signals

For each vehicle folder, determine:

- Does it contain exactly one `.bus` file, or multiple?
- Does it contain any `.ovh` files?
- Is it under `Vehicles/AI/` or similar subfolder?
- Does it contain a `passengercabin.cfg` with a `[drivpos]`?

**Deliverable:** A subsection titled "Folder Signals" with a table:

| Folder trait | Count | Correlation with playability |
| :--- | ---: | :--- |

---

## 5. Correlation Analysis

For each signal, compute:

- **Sensitivity:** of the vehicles flagged by the signal, what
  fraction are actually non-playable?
- **Specificity:** of the vehicles NOT flagged by the signal, what
  fraction are actually playable?

Since ground truth is not available for the full corpus, use
**intersection analysis** instead:

- Signals that co-occur across 100% of a filename class are
  candidates for that class's definition.
- Signals that are absent from a filename class are candidates for
  exclusion.

**Deliverable:** A subsection titled "Signal Correlation" with a
matrix:

| Signal A | Signal B | Co-occurrence | Interpretation |
| :--- | :--- | ---: | :--- |

Example rows:
- `ai_* filename` + `[scriptshare]` → 100% of `ai_*` have
  `[scriptshare]`?
- `*_trail` + `[couple_back]` → 100% of `*_trail` have
  `[couple_back]`?

---

## 6. Rule Proposal

Based on the correlation analysis, propose a rule system. Rules
must be:

- **Ordered by priority.** Higher-priority rules override lower.
- **Explainable.** Each rule cites its evidence.
- **Conservative.** False negatives (marking a playable as
  non-playable) are worse than false positives (marking a
  non-playable as playable). Default to "playable" when uncertain.
- **Extensible.** New signals can be added without invalidating
  existing ones.

Suggested structure:

```
1. If [couple_back] points to this file → non-playable (trailer)
2. If [scriptshare] is present → non-playable (inherits scripts)
3. If filename matches ^(ai_|AI_|ki_|KI_) → non-playable (AI)
4. If filename ends in _trail, _Trailer, _H, _V → non-playable
5. If referenced by .ailist and not by any [registration_*] → likely AI
6. If [registration_free] is present → playable
7. If [kmcounter_init] is present → playable
8. Default → playable
```

The rules above are illustrative. Derive the actual rules from your
evidence. Justify each rule with the correlation analysis.

**Deliverable:** A subsection titled "Rule System" with:

- The ordered rule list, one per line
- For each rule: evidence, sensitivity, specificity, counterexamples
- A decision tree diagram (ASCII or Mermaid) showing the flow

---

## 7. Validation

Rules must be validated against a sample of known playable and
known non-playable vehicles.

### 7.1 Known Samples

Identify:

- **Known playable:** vehicles that ship with OMSI 2 or well-known
  mods, in folders named after the vehicle with no AI/trailer
  markers. Example candidates: `MAN_SD200`, `MAN_NG272`,
  `MAN_GN92`.
- **Known non-playable:** vehicles in `Vehicles/AI/`, `ai_*.bus`
  files, `*_trail.bus` files.

For each known sample, run the rules and record:
- Which rule fired
- Whether the classification matches expectation

### 7.2 Counterexample Search

Search for cases where a rule might fire wrongly:

- Any `ai_*` file with `[registration_free]` (playable signal in
  an AI-named file)?
- Any non-`ai_` file with `[scriptshare]` (suggests trailer but
  named normally)?
- Any file with `[couple_back]` that also has `[drivpos]` (a
  playable tractor with trailer references)?

**Deliverable:** A subsection titled "Validation" with:

- The known-sample table
- The counterexample table
- A confidence statement about the rule system

---

## 8. Ground-Truth Recommendation

The rule system is only as good as its validation. Recommend how
to obtain ground truth:

- **Option A:** Enumerate the OMSI vehicle selection menu manually
  and record the playable set.
- **Option B:** Find the file OMSI uses to populate the menu (may
  be under `Vehicles/` or in `options.cfg`).
- **Option C:** Use community documentation to build a partial
  ground-truth set.

Recommend one and explain the tradeoffs. Do not attempt to obtain
ground truth yourself if it requires running OMSI.

**Deliverable:** A subsection titled "Ground-Truth Recommendation"
with the recommended option and reasoning.

---

## 9. Open Questions

List any questions the investigation could not resolve. For each:
- The question
- Why it matters
- What evidence would resolve it

---

## 10. Output Format

The output file `docs/src/architecture/playability-rules.md` must
contain:

```markdown
# 🎯 Vehicle Playability Rules

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Proposal — awaiting review |
| **Rule Version** | 1 (proposed) |
| **Last Updated** | YYYY-MM-DD |

## 📖 Overview
<What problem this solves and why the extension rule is
insufficient.>

## 🔍 Signal Enumeration
<All subsections from Section 4.>

## 📊 Correlation Analysis
<From Section 5.>

## 🎯 Rule System
<Ordered rules from Section 6, with evidence.>

## 🧪 Validation
<From Section 7.>

## 🎓 Ground-Truth Recommendation
<From Section 8.>

## ❓ Open Questions
<From Section 9.>

## 📝 Provenance

- Files inspected: <list>
- Techniques used: <list>

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents
- [Catalogue Schema](catalogue-schema.md)
- [`bus.md`](../formats/text/vehicles/bus.md)
- [`misc-types.md`](../formats/text/misc-types.md)
```

---

## 11. Reporting Cadence

Report after each of these checkpoints:

1. Signal enumeration (Section 4)
2. Correlation analysis (Section 5)
3. Rule proposal (Section 6)
4. Validation (Section 7)
5. Ground-truth recommendation (Section 8)

Do not proceed without confirmation.

Format:

```
## Playability Rules — Checkpoint N

**Checkpoint:** <name>
**Findings:** <summary>
**Rule implications:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 12. Exit Criteria
- ✅ All signals enumerated (filename, keyword, cross-reference,
  folder).
- ✅ Correlation matrix populated.
- ✅ Rule system proposed with ordered rules and evidence.
- ✅ Validation performed against known samples.
- ✅ Counterexamples documented.
- ✅ Ground-truth recommendation provided.
- ✅ `docs/src/architecture/playability-rules.md` exists.
- ✅ No OMSI binary was inspected.
- ✅ No OMSI asset was committed.
- ✅ No Rust code was written.

When finished, respond with a summary containing:

- Number of signals enumerated
- Number of rules proposed
- Validation pass rate against known samples
- Number of counterexamples found
- Recommended ground-truth option

Then respond with the exact string:
"PLAYABILITY RULES COMPLETE — ready for schema integration"