# Translating OPTS

| Metadata | Value |
| :--- | :--- |
| **Status** | German and English complete and tested; further languages are added through the source tables (pull request); loading language packs from the user directory is **not implemented** |
| **Spec** | [expandability, section 6](../roadmap/phase-9/expandability.md#6-other-registries-and-services) |

## Where the texts are

All user-visible text lives in Fluent-subset tables (`.ftl`), addressed by key:

| Area | English | German |
| :--- | :--- | :--- |
| Widgets, dialogs, theme | `crates/opts-gui/src/l10n/en.ftl` | `de.ftl` |
| Plan a drive (wizard) | `wizard_en.ftl` | `wizard_de.ftl` |
| Shell, Home, first run, Help, pause, summary | `crates/opts-game/src/shell/l10n/shell-en.ftl` | `shell-de.ftl` |
| Settings (titles, descriptions, options) | `crates/opts-game/src/settings_registry/messages_en.ftl` | `messages_de.ftl` |

English is the source language. The subset supports `key = text`, `{ $variable }`, `{ other-key }` references and plural selectors (`{ $count -> [one] ... *[other] ... }`). Map and vehicle descriptions are *content*, shown as written and not translated.

## Adding a language, step by step

1. Copy the four English tables to `xx` files (`xx` = the language tag, for example `fr`) next to them and translate every value; **keep every key and every `$variable`**.
2. Register the tables where the German ones are loaded (`Localization::new`, `shell::extend_localization_into`, `settings_registry::add_messages`) and add the language to the `LANGUAGES` options of `interface.language` in `settings_registry/builtin.rs`. Plural rules other than `one`/`other` need `fluent::plural_category` extended.
3. Run the tests that guard translations:
   - `cargo test -p opts-game --test localisation` fails for a missing key, a key without English source, a lost or added variable, an empty message, and for a German message identical to the English one (names and symbols are exempt); copy the checks for the new language.
   - `cargo test -p opts-gui l10n` checks key tables, fallback (locale, language, English, then a visible `⟦key⟧`), plurals and the ban on string literals outside the key tables.
   - `cargo test -p opts-game --test a11y_matrix text_survives` lays every screen out in the pseudo-locale `en-XA` (English with accents and 40 % more text) and in German at three window sizes: nothing may be cut, overlap or leave the window. German is about 30 % longer than English, so a language that passes there has room.
4. Look at the result: `OPTS_LOCALE=xx cargo run -p opts-game` (the environment variable chooses the language for one run).
5. Open a pull request with the four files. Do not translate the exact wording of *Feature not available yet!* by changing its key; translate the value.

## Rules for translators

Sentence case; units after the number with a thin space; no all-caps; keep a message short enough for a button (about 24 characters) unless it is a description; do not build sentences from fragments in code (use whole messages with variables); a shortcut or key name stays as printed on the keyboard.

## Not done

- Loading `<config>/opts/languages/<tag>.ftl` at start-up (community packs without recompiling). `Localization::extend` is the mechanism it would use; the missing part is the directory scan, the validation report in the Workshop and the language list in Settings.
- A review by native speakers of the German texts.
