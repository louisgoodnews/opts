# Translating OPTS

| Metadata | Value |
| :--- | :--- |
| **Status** | German and English are built in, complete and tested. **Any other language is a language pack**: plain text files in a folder, no Rust and no recompiling ([M-0083](../roadmap/phase-9/M-0083.md)) |
| **Spec** | [Localisation packs](../architecture/localisation-packs.md) (format decision, schema, validation codes), [expandability, section 6](../roadmap/phase-9/expandability.md#6-other-registries-and-services) |

## Where the texts are

All user-visible text lives in Fluent-subset tables (`.ftl`), addressed by key:

| Area | English | German |
| :--- | :--- | :--- |
| Widgets, dialogs, theme | `crates/opts-gui/src/l10n/en.ftl` | `de.ftl` |
| Plan a drive (wizard) | `wizard_en.ftl` | `wizard_de.ftl` |
| Shell, Home, first run, Help, pause, summary | `crates/opts-game/src/shell/l10n/shell-en.ftl` | `shell-de.ftl` |
| Settings (titles, descriptions, options) | `crates/opts-game/src/settings_registry/messages_en.ftl` | `messages_de.ftl` |
| Heads-up display | `crates/opts-game/src/hud/l10n/hud-en.ftl` | `hud-de.ftl` |

English is the source language. The subset supports `key = text`, `{ $variable }`, `{ other-key }` references and plural selectors (`{ $count -> [one] ... *[other] ... }`). Map and vehicle descriptions are *content*, shown as written and not translated.

## Add a language: the short version

```sh
opts-tools l10n new fr --name French --native-name Français --author "Your Name"
#   -> <user data dir>/lang/fr/pack.json + gui.ftl, wizard.ftl, shell.ftl, settings.ftl, hud.ftl
# translate the text after "=" in the .ftl files, keep every key and every { $variable }
opts-tools l10n check <user data dir>/lang/fr          # errors and gaps, with file and line
# start the game: Settings > Interface > Language > Français   (or "Reload languages")
```

A pack in the user folder is found at start-up and by *Reload languages*; nothing is compiled. A pack with gaps shows English for the gaps. A broken file only loses its broken messages; a broken pack never stops the game.

Where is the folder? `opts-tools l10n stats` prints it, and so does *Settings → Interface → Language* below the list (`~/.local/share/opts/lang/` on Linux; the environment variable `OPTS_LANG_DIR` points OPTS at another folder, for testing a pack without installing it). Each language has its own folder named after its tag:

```text
<user data dir>/lang/fr/pack.json        who, what language, direction, licence
<user data dir>/lang/fr/*.ftl            messages (any file names)
<user data dir>/lang/fr/*.json           messages in the JSON form (optional, see below)
```

## Workflow for a human translator

1. **Create the pack:** `opts-tools l10n new <tag> --name <English name> --native-name <name in the language> --author "<you>"`. `<tag>` is the language code: `fr`, `pt-BR`, `zh-Hans`.
2. **Edit `pack.json`** if needed: `direction` (`ltr`; `rtl` is not supported by the interface yet), `version`, `authors`. Leave `license` as `MIT OR Apache-2.0`.
3. **Translate** the `.ftl` files: the scaffold has the English text after each `=`; replace it. Comments (`# ...`) above a message say what it is and which variables it takes. Delete a message you do not want to translate yet; English shows instead.
4. **Check:** `opts-tools l10n check <pack folder>` prints `file:line: error[code]: what is wrong` for every problem and a summary line (`87.3 % translated`). Fix errors; read warnings (`untranslated-copy`, `suspicious-length`, `plural-forms`).
5. **Look at it:** start the game (`OPTS_LOCALE=fr cargo run -p opts-game` chooses it for one run, or pick it in Settings). Click through the screens; long texts that do not fit are the usual finding.
6. **Share it:** zip the folder, or open a pull request that adds it (see licence below).

### The `.ftl` syntax you need

```ftl
# Comment lines start with a hash.
widget-clear = Effacer
hello = Bonjour { $name }
widget-items =
    { $count ->
        [one] { $count } élément
       *[other] { $count } éléments
    }
```

- One message is `key = text`. A longer text continues on indented lines.
- `{ $name }` is a variable; `{ other-key }` inserts another message. Keep both **exactly**: the checker rejects a lost, added or misspelt variable.
- A plural message lists the forms of *your* language: `one`, `other`, and for Russian, Polish and others also `few`, `many` (the check warns about missing ones; `l10n export` lists them). `*[other]` is the form used when no other matches and is required. A number in brackets, `[0]`, matches exactly that number.
- A literal brace is written `{ "{" }`.

## Workflow for an AI agent

Give the agent the JSON template, not the repository. Everything it needs is in the file.

```sh
# 1. template for French: every English message with context, variables and the plural forms of French
opts-tools l10n export --lang fr --format json --out fr.json
#    (use --missing to get only what a pack still lacks, --pack <dir> to include an existing pack)

# 2. the agent fills "text" (and, for plural messages, every entry of "forms") and the "pack" object
#    (code, name, native_name, version, authors); an empty "text" means "not translated yet"

# 3. verify, with machine-readable output the agent can read and act on
opts-tools l10n check fr.json --json            # exit code 1 when there are errors
opts-tools l10n import fr.json                  # writes <user data dir>/lang/fr/ (pack.json + .ftl)
opts-tools l10n check <user data dir>/lang/fr --json --min-percent 95

# 4. in the running game: Settings > Interface > Language > Reload languages
#    or over the control protocol: l10n.reload, l10n.set {"code":"fr"}, l10n.report {"code":"fr"}
```

Rules to give the agent (they are also in the checker's messages):

- Translate only `text` and `forms`; never change keys, `variables` or `source`.
- Keep every `{ $variable }` and `{ other-key }` of `source`; do not translate them or add new ones.
- Provide **all** forms of a plural message or none: a half-filled `forms` object is an error.
- Keep texts about as long as the English: a button is about 24 characters.
- Sentence case, units after the number with a thin space, no all-caps, shortcut and key names as printed on the keyboard.
- Do not invent keys; do not leave English text in a translated field (`untranslated-copy` counts it as not done).
- Say in `authors` that the text is machine-made and name the reviewer, if any.

### The JSON form `opts-l10n/1`

```json
{
  "format": "opts-l10n/1",
  "language": "fr",
  "base_version": "1a2b3c4d",
  "pack": { "code": "fr", "name": "French", "native_name": "Français", "direction": "ltr",
            "version": "0.1.0", "authors": ["Jane Doe"], "license": "MIT OR Apache-2.0",
            "base_version": "1a2b3c4d" },
  "messages": {
    "widget-clear": {
      "context": "Tooltip of a text field's clear button.",
      "file": "gui",
      "variables": [],
      "source": "Clear",
      "text": "Effacer"
    },
    "widget-items": {
      "file": "gui",
      "variables": ["count"],
      "variable": "count",
      "source_forms": { "one": "{ $count } item", "other": "{ $count } items" },
      "forms": { "one": "{ $count } élément", "many": "{ $count } d'éléments", "other": "{ $count } éléments" }
    }
  }
}
```

It is lossless: `export` then `import` returns exactly the tables the game ships (a test proves it). A plain `{ "key": "text" }` object is accepted as well. `source`, `source_forms`, `context`, `variables` and `file` are information for the translator and ignored on import (`file` only chooses the `.ftl` file name). A JSON file may also sit directly in a pack folder next to the `.ftl` files; it is checked the same way.

## Pack schema (`pack.json`)

| Field | Required | Meaning |
| :--- | :--- | :--- |
| `code` | yes | Language tag, equal to the folder name |
| `name` | yes | English name |
| `native_name` | yes | Name in the language itself (the button text) |
| `direction` | no | `ltr` (default) or `rtl` (listed, not selectable yet) |
| `version` | yes | Version of the pack |
| `authors` | recommended | Names or handles; say if machine-made |
| `license` | yes | `MIT OR Apache-2.0` |
| `base_version` | no | English revision the strings were made against; `l10n stats` prints the current one. If it differs the pack is marked *outdated* (a warning); `l10n export --lang <tag> --pack <dir> --missing` lists what is new |

Every check, with its code, is in the [validation table](../architecture/localisation-packs.md#3-validation).

## Licence

Contributed text is licensed as the project: **MIT OR Apache-2.0**, at the user's choice (`LICENSE-MIT`, `LICENSE-APACHE`). By putting `"license": "MIT OR Apache-2.0"` in `pack.json` and sending the pack, you confirm that you wrote the text (or have the right to license it that way) and that it may be shipped with the game. Do not copy text from other programs or translation memories you have no right to. A pack you keep for yourself in your own user folder carries no obligation.

## Improving German or English

The two built-in languages live in the source tables above; fix them with a pull request that edits the `.ftl` files (keep `de` and `en` in step), or override single German messages locally with a pack named `de`.

To add a **built-in** language (compiled into the program) instead of a pack: copy the five English tables to `xx` files, register them where the German ones are loaded (`Localization::new`, `shell::extend_localization_into`, `settings_registry::add_messages`, `hud::add_messages`, the list in `opts-core/src/l10n/builtin.rs`) and add the tests. Prefer a pack unless the language should ship in every build.

## Tests that guard translations

- `cargo test -p opts-core l10n` covers the parser, the checks, packs and the JSON round trip.
- `cargo test -p opts-game --test localisation` fails for a missing key, a key without English source, a lost or added variable, an empty message, a German message identical to the English one, and when the list of tables in `opts-core` and the loaded tables drift apart.
- `cargo test -p opts-game --test language_packs` loads packs from a folder: discovery, override, per-key fallback, broken files, reload and the control methods.
- `cargo test -p opts-gui l10n` checks key tables, fallback, plurals and the ban on string literals outside the key tables.
- `cargo test -p opts-game --test a11y_matrix text_survives` lays every screen out in the pseudo-locale `en-XA` (English with accents and 40 % more text) and in German at three window sizes: nothing may be cut, overlap or leave the window.

## Rules for translators

Sentence case; units after the number with a thin space; no all-caps; keep a message short enough for a button (about 24 characters) unless it is a description; do not build sentences from fragments in code (use whole messages with variables); a shortcut or key name stays as printed on the keyboard. Translate the value of *Feature not available yet!*, never its key.

## Not done

- Right-to-left layout and fonts for scripts beyond Latin, Greek and Cyrillic: such packs are listed with a red note ([architecture](../architecture/localisation-packs.md#3-validation)).
- Number, date and unit formatting per language.
- A review by native speakers of the German texts.
