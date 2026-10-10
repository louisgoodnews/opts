# 🔤 Encoding Corruption Repair

You are repairing UTF-8 encoding corruption across the OPTS
documentation. Several Markdown files contain "mojibake" — UTF-8
bytes that were decoded as CP1252 (Windows-1252) and re-encoded as
UTF-8. The result is valid UTF-8 that renders as gibberish.

Example corruption found in `docs/src/formats/text/money.md`:

| Corrupted | Intended |
| :--- | :--- |
| `тАФ` | `—` (em dash) |
| `ЁЯЯв` | 🟢 |
| `тЭУ` | ❓ |
| `ЁЯУЦ` | 📖 |
| `тЬЕ` | ✅ |

This is a **surgical repair task**. You are not rewriting content.
You are not refactoring. You are restoring the intended characters.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Scope
All `.md` files in:
- `docs/src/`
- `docs/crates/`
- `docs/dev/`
- `dev/`
- Repository root (`*.md`)

### Excluded
- Any file in `target/`
- Any file in `.git/`
- Any file in `node_modules/`
- Any generated file (mdBook `book/`, etc.)

---

## 2. Root Cause

The corruption follows this exact transformation:

```
Original byte sequence:    0xE2 0x80 0x94   (UTF-8 for em dash —)
Decoded as CP1252:         â€"             (three characters)
Re-encoded as UTF-8:       0xC3 0xA2 0xE2 0x82 0xAC 0xC2 0x9D
```

The reverse transformation recovers the original:

```
Corrupted text (UTF-8):    â€"
Encode as CP1252:          0xE2 0x80 0x94
Decode as UTF-8:           —                (em dash —)
```

This round-trip works because CP1252 is a single-byte encoding
where each byte maps to a character, and the original corruption
preserved that byte sequence in the intermediate representation.

---

## 3. Guardrails

### ❌ Forbidden

- Do NOT modify source code (`.rs` files).
- Do NOT modify `.toml`, `.yml`, `.gitignore`, or any non-Markdown
  file.
- Do NOT touch files under `target/` or `.git/`.
- Do NOT touch any file inside an OMSI installation.
- Do NOT "helpfully" fix other issues found while sweeping (broken
  links, spelling, formatting). Those belong in separate passes.
- Do NOT commit until the sweep is verified.

### ✅ Permitted

- Read every Markdown file in scope.
- Apply the round-trip transformation to detect corruption.
- Apply the reverse transformation to fix corruption.
- Create a report of what was changed.
- Commit the repair once verified.

### ⚠️ Preserve

- File line endings (LF vs CRLF) — do not normalize.
- Trailing whitespace — leave it, unless it's part of a corrupted
  sequence.
- Code fences and their contents — but DO fix corruption inside
  them if present.
- YAML front matter, if any.
- Existing Markdown structure.

---

## 4. Methodology

### Phase A — Detection

Do not modify anything in this phase.

#### Stage A1 — Signatures

Grep for these known mojibake signatures across all files in scope:

```
тА    (em dash and quotes family)
ЁЯ    (most emoji)
тЭ    (some symbols)
├Ч    (multiplication sign)
тЬ    (checkmark)
┬╖    (middle dot)
┬░    (degree)
├╝    (ü)
├д    (ä)
├╢    (ö)
├Я    (ß)
```

Also grep for the individual high-byte signatures:
```
├    (0xC3 seen as CP1252)
т    (0xE2 seen as CP1252)
Ё    (0xD0/0xD1 seen as CP1252)
```

If a file has zero matches for any of these, it is clean — skip it.

#### Stage A2 — Round-Trip Verification

For each file with matches, apply the round-trip transformation to
the whole file:

```python
def detect_corruption(text: str) -> str | None:
    try:
        cp1252_bytes = text.encode('cp1252')
    except UnicodeEncodeError:
        return None  # contains characters not representable in CP1252
    try:
        recovered = cp1252_bytes.decode('utf-8')
    except UnicodeDecodeError:
        return None  # byte sequence is not valid UTF-8
    return recovered
```

If the recovered text differs from the input text and contains
plausible Markdown/emoji/dash content, the file is corrupted.

If the recovered text contains control characters, non-printable
sequences, or looks worse than the input, DO NOT apply it. Report
the file as "needs manual review."

#### Stage A3 — Detection Report

Produce `docs/src/meta/encoding-audit-YYYY-MM-DD.md`:

| File | Signatures Found | Round-Trip Valid | Category |
| :--- | :--- | :--- | :--- |
| `path/to/file.md` | `тА, ЁЯ` | ✅ | Fixable |
| `path/to/other.md` | `├Ч` | ⚠️ | Ambiguous |
| `path/to/clean.md` | — | N/A | Clean |

Categories:
- **Clean** — no signatures found.
- **Fixable** — round-trip produces a valid improvement.
- **Ambiguous** — signatures found but round-trip fails or produces
  questionable output.
- **Manually reviewed** — human has confirmed the fix is correct.

Report counts:
- Total files in scope
- Files clean
- Files fixable
- Files ambiguous (with reasons)

**STOP and report the detection results before proceeding to Phase B.**

---

### Phase B — Repair

Only run after the detection report is reviewed and approved.

#### Stage B1 — Fix Order

Fix files in this order, one batch at a time, verifying after each:

1. `docs/src/formats/text/money.md` (known corrupted — sanity check the method)
2. `docs/src/formats/` (all specs)
3. `docs/src/roadmap/` (milestone files)
4. `docs/src/adr/` (architecture decisions)
5. `docs/src/meta/` (audits and reports)
6. `docs/crates/` (crate docs)
7. `docs/dev/` and `dev/` (board and prompts)
8. Repository root (`README.md`, `CONTRIBUTING.md`, etc.)

For each batch:
1. Apply the round-trip fix.
2. Run `git diff` on the batch.
3. Spot-check 3–5 files manually — did the emoji come back?
   Do the dashes look right?
4. If any file looks worse, revert it and mark it Ambiguous.
5. Commit the batch with a message like:
   `docs: fix UTF-8 mojibake in docs/src/formats/ (N files)`

#### Stage B2 — Verify Each Batch

After each batch:

1. Confirm the corrupted signatures no longer appear:
   ```bash
   grep -rn "тА\|ЁЯ\|тЭ" docs/src/formats/
   ```
   Only acceptable matches are inside code blocks that legitimately
   show mojibake (rare).

2. Spot-check rendering: run `mdbook build docs` if `mdbook` is
   available, or open a few files in a Markdown preview.

3. Verify links still resolve (the sweep should not have changed
   link paths — it only changed characters).

#### Stage B3 — Ambiguous Cases

For files marked Ambiguous in Stage A3:

1. Open each file.
2. Manually inspect the corrupted sequences in context.
3. If the intent is obvious (e.g., `тАФ` between two words → em dash),
   fix manually and move the file to Manually Reviewed.
4. If the intent is unclear, leave the file as-is and document it
   in the final report.

Do NOT guess. A file with one unresolved corruption is better than
a file with a well-intentioned wrong fix.

#### Stage B4 — Cross-Check

After all batches are complete:

1. Grep the entire scope for remaining signatures.
2. Any file still matching is either:
   - Legitimately showing mojibake as an example (e.g., in
     `sco-sli-audit.md` where a corrupted keyword was observed)
   - Genuinely corrupted but missed
   - Ambiguous and left for review

3. Document the classification in the final report.

---

## 5. Reference Mapping

The following mapping table is provided as a cross-check. Do NOT
use it as the primary fix method — use the round-trip. Use this
table to verify that the round-trip produces sensible results.

| Corrupted | Intended | Unicode |
| :--- | :--- | :--- |
| `тАФ` | — | U+2014 EM DASH |
| `тАУ` | – | U+2013 EN DASH |
| `тАШ` | ' | U+2018 LEFT SINGLE QUOTE |
| `тАЩ` | ' | U+2019 RIGHT SINGLE QUOTE |
| `тАЬ` | " | U+201C LEFT DOUBLE QUOTE |
| `тАЭ` | " | U+201D RIGHT DOUBLE QUOTE |
| `тАж` | … | U+2026 HORIZONTAL ELLIPSIS |
| `тЬЕ` | ✅ | U+2705 WHITE HEAVY CHECK MARK |
| `тЭУ` | ❓ | U+2753 BLACK QUESTION MARK ORNAMENT |
| `├Ч` | × | U+00D7 MULTIPLICATION SIGN |
| `┬╖` | · | U+00B7 MIDDLE DOT |
| `┬░` | ° | U+00B0 DEGREE SIGN |
| `├╝` | ü | U+00FC LATIN SMALL LETTER U WITH DIAERESIS |
| `├д` | ä | U+00E4 LATIN SMALL LETTER A WITH DIAERESIS |
| `├╢` | ö | U+00F6 LATIN SMALL LETTER O WITH DIAERESIS |
| `├Я` | ß | U+00DF LATIN SMALL LETTER SHARP S |
| `├й` | é | U+00E9 LATIN SMALL LETTER E WITH ACUTE |
| `├и` | è | U+00E8 LATIN SMALL LETTER E WITH GRAVE |
| `├а` | à | U+00E0 LATIN SMALL LETTER A WITH GRAVE |
| `┬й` | © | U+00A9 COPYRIGHT SIGN |
| `┬о` | ® | U+00AE REGISTERED SIGN |
| `┬░` | ° | U+00B0 DEGREE SIGN |

Emoji (partial — the round-trip handles all of them):

| Corrupted | Intended |
| :--- | :--- |
| `ЁЯУЦ` | 📖 |
| `ЁЯУЪ` | 📚 |
| `ЁЯУЭ` | 📝 |
| `ЁЯУК` | 📊 |
| `ЁЯУЛ` | 📋 |
| `ЁЯФд` | 🔤 |
| `ЁЯФЧ` | 🔗 |
| `ЁЯзк` | 🧪 |
| `ЁЯЯв` | 🟢 |
| `ЁЯЯб` | 🟡 |
| `ЁЯФ┤` | 🔴 |
| `ЁЯЫФ` | 🛔 |
| `ЁЯШВ` | 😂 |

---

## 6. Final Report

Produce `docs/src/meta/encoding-repair-YYYY-MM-DD.md` with:

### Summary
- Total files scanned
- Files with corruption detected
- Files successfully repaired
- Files left for manual review
- Files confirmed clean

### Batches
For each batch:
- Number of files fixed
- Commit hash
- Any files reverted (with reason)

### Ambiguous Cases
- File path
- Corrupted signature observed
- Why the round-trip failed or produced questionable output
- Recommended action

### Verification
- `grep` results confirming clean state
- mdBook build status (if available)
- Any residual signatures and their classification

### Root Cause Prevention
- Recommend adding an encoding check to CI:
  ```yaml
  - name: Check UTF-8 encoding
    run: |
      find docs/ -name "*.md" -exec sh -c \
        'file "$1" | grep -q "UTF-8" || echo "Not UTF-8: $1"' _ {} \;
  ```
- Recommend a pre-commit hook or editor config:
  ```
  # .editorconfig
  [*.md]
  charset = utf-8
  ```
- Document in `CONTRIBUTING.md` under "Documentation Standards"

### Clean-Room Attestation
- "No OMSI binaries were inspected."
- "No GPL source code was read."
- "No OMSI assets were committed."

---

## 7. Exit Criteria

- ✅ Detection report produced and reviewed.
- ✅ All Fixable files repaired.
- ✅ All batches committed separately.
- ✅ All Ambiguous files documented.
- ✅ `grep` confirms no unexpected signatures remain.
- ✅ Final report exists at `docs/src/meta/encoding-repair-YYYY-MM-DD.md`.
- ✅ CI or pre-commit recommendation included.
- ✅ `CONTRIBUTING.md` updated with encoding guidance.
- ✅ The freeze report's gap is acknowledged in the final report.

When finished, respond with the final report and the exact string:
"ENCODING REPAIR COMPLETE — ready for P-001"