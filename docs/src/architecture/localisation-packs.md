# Localisation Packs

Milestone [M-0083](../roadmap/phase-9/M-0083.md) (field report R-25). Builds on the localisation
scaffold ([M-0061](../roadmap/phase-9/M-0061.md)) and the translation tests
([M-0065](../roadmap/phase-9/M-0065.md)); the contributor-facing guide is
[Translating OPTS](../reference/translating.md).

**Goal:** a person or an AI agent adds or improves a language by writing plain text files and
dropping them in a folder. No Rust, no recompiling. The game finds them, checks them, lists them in
*Settings → Interface → Language*, and a broken pack never stops it from starting.

Code: `crates/opts-l10n/src/` (format, checks, packs, interchange; no engine),
`crates/opts-gui/src/l10n/` (`Localization`, the pack overlay), `crates/opts-game/src/language_packs.rs`
(start-up scan, control methods), `crates/opts-tools/src/l10n.rs` (`opts-tools l10n ...`).

## 1. Decision: which file format

Requirements, in order of weight:

1. **Humans can edit it** in any text editor, and a diff of a pull request shows what changed.
2. **AI agents can fill it** reliably: a template with the source text and context next to the field
   to fill, no hidden syntax to get wrong, and a checker whose output the agent can read and act on.
3. **Plurals and variables** are first class (`{ $count } items`; Russian has four plural forms).
4. **Validation** with a precise file and line for every problem.
5. Tooling and ecosystem (editors, translation platforms), and the cost of keeping it.

| Format | Humans | AI agents | Plurals and variables | Validation / line numbers | Diffs | Tooling | Verdict |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Fluent `.ftl`** (today) | `key = text`, comments, multi-line; very readable | good; the syntax is small, but a plural selector is easy to get wrong | built in: `{ $var }`, selectors, references between messages | we own the parser: exact line per message and per fault | one line per message | Weblate, Pontoon, editor plugins; the runtime format already | **runtime format** |
| **Flat JSON** `{key: text}` | readable but no comments, no multi-line, quote escaping | excellent (every LLM emits valid JSON) | none; plurals invented per project | JSON parsers report one syntax error and stop; no per-key line | one line per key, but no context | universal | **accepted as an input shape** |
| **JSON with metadata and plural forms** | verbose | **best**: source text, context, variables and plural forms next to the field to fill; structured plural forms | plural forms as an object, variables listed | JSON syntax plus our checks; line of each key found by search | stable field order makes diffs small | universal | **interchange form `opts-l10n/1`** |
| **gettext PO / XLIFF** | PO is readable; XLIFF is XML noise | PO fine, XLIFF poor | PO plurals use C expressions (`nplurals=2; plural=(n != 1)`) and `msgid` is the English text, not a key | mature (`msgfmt -c`) | good (PO) | the widest translator tooling | not adopted: our keys are identifiers and our variables use Fluent placeholders; a converter can be written later because the JSON form is lossless |
| **TOML** | pleasant for flat tables | good | none; plurals need sub-tables | line numbers from the parser, but multi-line and quoting rules differ from our messages | good | Rust friendly | not adopted: no advantage over Fluent for text, no plural story |

**Decision.**

- **Fluent (subset) stays the runtime format.** All shipped tables already use it, the parser is ours
  and gives a line for every fault, plurals and variables are part of the syntax, and English and
  German are tested through it. Changing it would touch 1,000 messages for no gain.
- **A lossless JSON form, `opts-l10n/1`, is offered for export and import** (`opts-tools l10n export
  --format json`, `l10n import`). It is what an AI agent or a script should fill: each message is an
  object with `source`, `context`, `variables` and the field to fill, plural messages carry their
  forms as an object keyed by the plural categories *of the target language*. A test proves the
  round trip (`export` then `import` returns exactly the shipped English and German tables).
- A pack may **also contain JSON string files** directly: the game reads `*.json` next to `*.ftl`
  with the same checks, so an agent that returns a filled template can drop it in without a
  conversion step.
- PO/XLIFF and TOML are *not* supported. If a translation platform needs PO later, a converter on top
  of the JSON form is a small tool, not a format decision.

Why this serves both audiences: a human opens `gui.ftl`, sees `# comment`, `key = text`, translates
and runs `opts-tools l10n check`; an agent calls `l10n export --format json --missing`, fills the
`text`/`forms` fields, saves it as `strings.json` in the pack folder and runs the same check, whose
`--json` output lists every fault with `file`, `line`, `key`, `code` and a message that says what to
change. Both end in the same loader.

## 2. Pack layout and discovery

```text
<user data dir>/lang/<code>/pack.json     identity (required)
<user data dir>/lang/<code>/*.ftl         messages, any file names
<user data dir>/lang/<code>/*.json        messages, interchange form or flat map
```

The user data directory is the one the game uses for its other data (`~/.local/share/opts` on Linux;
`opts-tools l10n stats` prints the folder). The folder name is the language tag and must equal
`code` in `pack.json`.

**Discovery** happens once at start-up (after every plugin has added its English messages) and on
demand: *Settings → Interface → Reload languages* and the control method `l10n.reload`. Reload replaces
the pack messages as a whole; if the active language is gone the game shows English.

**Overlay.** A pack's messages are looked up *before* the built-in table of the same tag: a pack named
`de` overrides single German messages, a pack named `fr` adds French. A pack never changes English
(unknown keys are skipped, see below).

**Fallback chain** (per message, not per pack): `selected tag` → `its language` (`pt-BR` → `pt`) →
`en` → the visible marker `⟦key⟧`. A pack with gaps therefore shows English for the gaps. Missing keys
are listed by `l10n.report` and counted in the language list.

Limits that keep a hostile or broken folder harmless: 64 packs, 64 string files per pack, 2 MiB per
file, 2,000 characters per message, UTF-8 only, hidden folders and files ignored, nothing outside the
pack folder is read.

### `pack.json` reference

```json
{
  "code": "fr",
  "name": "French",
  "native_name": "Français",
  "direction": "ltr",
  "version": "0.1.0",
  "authors": ["Jane Doe", "an AI assistant, reviewed by Jane Doe"],
  "license": "MIT OR Apache-2.0",
  "base_version": "1a2b3c4d"
}
```

| Field | Required | Meaning |
| :--- | :--- | :--- |
| `code` | yes | Language tag: 2-3 lower-case letters, optional subtags (`fr`, `pt-BR`, `zh-Hans`); equals the folder name; `en-XA` is reserved for the test pseudo-locale |
| `name` | yes | English name of the language |
| `native_name` | yes | Name in the language itself; it is the button text in the language list |
| `direction` | no (`ltr`) | `ltr` or `rtl`. Right-to-left packs are listed but cannot be selected yet |
| `version` | yes | Version of the pack (free text) |
| `authors` | no, but warned | Names or handles; say when a translation was machine-made |
| `license` | yes | Expected `MIT OR Apache-2.0`: contributed text is licensed as the project; another value is a warning |
| `base_version` | no | The English revision the strings were made against (`opts-tools l10n stats` prints the current one); a mismatch marks the pack *outdated* |

## 3. Validation

Every problem has a **severity**, a stable **code**, a **file**, a **line** (when known), the **key**
and a message that says what is wrong and what to do (the error-template shape: what, why, next
step). Errors skip the message (English shows instead); warnings keep it. Nothing panics, nothing
aborts: a bad file costs only its bad messages, a bad `pack.json` costs the pack.

Printed as `file:line: error[code]: message (key `key`)`, the shape compilers use, so an editor can
jump to the line.

| Code | Severity | Meaning |
| :--- | :--- | :--- |
| `pack-json` | error | `pack.json` missing, not JSON, or a field missing/wrong; the pack is not loaded |
| `read` | error / warning | file unreadable, not UTF-8, too large; too many files |
| `syntax` | error | unclosed `{`, text outside a message, unsupported placeable, selector without `*[default]`, JSON entry of the wrong shape |
| `unknown-key` | error | key not in English (typo or outdated) |
| `variable-mismatch` | error | a `{ $variable }` is missing or added compared with English |
| `unknown-reference` | error | `{ other-message }` points at a message that does not exist |
| `empty` | error | empty message (delete the key instead; in JSON an empty `text` means *not translated yet* and is skipped silently) |
| `too-long` | error | more than 2,000 characters |
| `bad-character` | error | control character, U+FFFD or the `⟦` gap marker |
| `duplicate-key` | warning | key defined twice (the later wins) |
| `untranslated-copy` | warning | identical to English prose; counts as *copied*, not translated |
| `suspicious-length` | warning | more than 2.5x the English length (it may not fit a button) |
| `plural-forms` | warning | a plural category of the language is missing (`few` for Russian) |
| `licence` | warning | `license` is not the project's |
| `outdated` | warning | `base_version` differs from the current English revision |
| `language-mismatch` | warning | a JSON file's `language` differs from the pack's `code` |
| `font-coverage` | warning | the text uses a script the bundled fonts (Latin, Greek, Cyrillic) lack |
| `rtl-unsupported`, `direction` | warning | right-to-left pack or text that is not declared right-to-left |

**Completeness** = messages that are present, valid and not an untouched copy of English *prose*
(two words or more), over all English messages. One-word texts may legitimately equal English (names,
loanwords), so they count as translated; every untouched sentence is a *copy* and counts as not done. A
freshly scaffolded pack therefore shows well under half, never a flattering 100 %.

**Right-to-left and fonts.** The interface lays text out left to right. A pack with
`"direction": "rtl"` (or Arabic/Hebrew text) is listed with a red note and cannot be selected until
right-to-left layout exists. Scripts outside Latin, Greek and Cyrillic (CJK, Thai, Devanagari, ...)
need a font the game does not bundle; the pack is listed with a red note and those characters show
as empty boxes. Both are *reported*, not hidden.

## 4. Tooling: `opts-tools l10n`

Machine-readable output everywhere (`--json`; `export --format json` always), exit codes for CI: `0`
fine, `1` errors found (or warnings with `--strict`, or completeness below `--min-percent`), `2` wrong usage.

| Command | What it does |
| :--- | :--- |
| `l10n export --lang <code> [--format json\|ftl] [--pack <dir>] [--missing] [--out <path>]` | Template with every English message, the current translation (built-in table, then `--pack`), context comments, variable lists and the plural categories of the language. `--missing` limits it to untranslated keys |
| `l10n check <pack dir \| file.json \| file.ftl> [--lang <code>] [--json] [--strict] [--min-percent N]` | Validation and completeness report with every issue |
| `l10n new <code> [--name N] [--native-name N] [--author A] [--root <dir>] [--force]` | Scaffold `<root>/<code>/` with `pack.json` and the English files to translate (default root: the user pack folder) |
| `l10n import <file.json> [--root <dir>] [--force] [--lenient]` | Turn a filled JSON template into a pack folder (`pack.json` from its `pack` object); refuses on errors unless `--lenient` |
| `l10n stats [--root <dir>] [--json]` | Per language: translated, copied, missing, errors, warnings, outdated; prints the current English revision |

The tool uses the same `opts-core` code as the game; a pack that passes `check` loads.

## 5. In the game

*Settings → Interface → Language* lists the built-in languages and every usable pack, each as a
button with the language's own name. Below a pack's button: percentage translated and authors, how
many texts still show English, *made for an older version*, the number of problems (red), the scripts
the fonts lack (red) and, for right-to-left packs, the red note that it cannot be selected (the button
is disabled with that reason). A note names the folder to put packs into. *Reload languages* scans
again and answers with a toast.

The stored value of `interface.language` is the tag. A tag that is no longer installed leaves the
game in English; the setting is not rewritten.

### Control protocol

| Method | Parameters | Result |
| :--- | :--- | :--- |
| `l10n.languages` | none | `{current, packs_dir, languages:[{code, name, native_name, direction, builtin, from_pack, authors, version, total, translated, copied, missing, percent, errors, warnings, outdated, missing_fonts, selectable, current, dir}], problems:[{folder, issues}]}` |
| `l10n.set` | `{code}` | `{current, previous}`; `1004` unknown language, `1007` right-to-left; saved like `settings.set` |
| `l10n.reload` | none | the `l10n.languages` result plus `usable`, `broken`, `errors` |
| `l10n.report` | `{code?}` (default: current) | `{language, missing_keys, copied_keys, issues:[{severity, code, file, line, key, message}], issue_lines}` |

## 6. Not done

- Right-to-left layout; fonts for further scripts (a pack could carry a font; not designed yet).
- Number, date and unit formatting per language; the plural rules cover the CLDR cardinal rules of
  the common languages for whole numbers only.
- A review process for contributed packs (who may ship a pack with the game): a pack in the user
  folder is trusted as text only, it cannot run code.
- Packs inside the repository (`assets/lang/`) shipped with the release: today shipped languages are
  the compiled tables.
