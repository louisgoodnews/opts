//! Settings and localisation commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "settings.list",
        "settings",
        "List the settings with their values (optionally one category)",
        "category:s@0?",
    ),
    c(
        "settings.get",
        "settings",
        "Value, default and options of one setting",
        "key:s@0",
    ),
    c(
        "settings.set",
        "settings",
        "Change a setting (validated like the settings page, saved)",
        "key:s@0 value:s@1 confirm:b?",
    )
    .result("the setting as `settings.get` shows it, plus `previous`, `applied` (it changed), `needs_restart` and, while it waits, `pending_confirm` and `confirm_within_s`")
    .doc("Applies and saves at once. A setting that the page protects with a 15 s prompt (`apply.revert_after_s` in `settings.list`: window mode, resolution) can instead be tried with `confirm: false`: it is applied but not saved and waits for `settings.confirm` (keep and save) or `settings.revert` (restore); after its time it is reverted by itself, like the page's prompt. Only one such change waits at a time (`1006 BUSY` otherwise); `confirm: false` on any other setting changes nothing about how it is applied. `needs_restart` is true when the change only takes effect after a restart."),
    c(
        "settings.reset",
        "settings",
        "Restore the default of a setting",
        "key:s@0 confirm:b?",
    )
    .doc("Like `settings.set` with the default value, including `confirm`."),
    c(
        "settings.reset_keys",
        "settings",
        "Restore the default of every key binding, or of one group",
        "group:s@0?",
    )
    .since("1.23")
    .result("`{changed, bindings:[{key, previous, value}]}`")
    .doc("`group` is `all` (default), `vehicle`, `drive`, `camera`, `map` or `ui`. Only bindings that differ from their default are written; the defaults of all actions are unique, so the order does not matter. The `controls.reset.*` rows of the Controls page do the same."),
    c(
        "settings.confirm",
        "settings",
        "Keep the risky setting change that waits and save it",
        "",
    )
    .since("1.7")
    .result("`{key, value, confirmed: true}`")
    .doc("Finishes a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits."),
    c(
        "settings.revert",
        "settings",
        "Undo the risky setting change that waits",
        "",
    )
    .since("1.7")
    .result("`{key, value, reverted: true}`")
    .doc("Restores the value from before a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits."),
    c(
        "settings.set_many",
        "settings",
        "Change several settings at once, all or none",
        "values:j@0",
    )
    .since("1.7")
    .result("`{count, changed: [{key, previous, value, needs_restart}], unchanged, needs_restart: [keys]}`")
    .doc("`values` is an object of setting key to value. The whole batch is validated first (unknown or unavailable keys, bad values, key bindings that clash inside the batch); if anything is refused nothing is applied and the answer is `-32602` with `data.errors: [{key, code, message}]` listing every refusal. Settings with a revert prompt are applied at once, without the `confirm` wait. Saved once at the end; at most 1000 entries."),
    c(
        "settings.diff",
        "settings",
        "The settings that differ from their defaults",
        "",
    )
    .since("1.7")
    .result("`{count, differences: [{key, value, default}]}`")
    .doc("Visible settings only (internal state such as the last map is left out)."),
    c(
        "settings.export",
        "settings",
        "A profile of the changed settings, to import later",
        "all:b?",
    )
    .since("1.7")
    .result("`{format: \"opts-settings\", version: 1, count, settings: {key: value}}`")
    .doc("Without `all` only the settings that differ from the defaults; with `all: true` every visible setting. Hand the result (its `settings` object) to `settings.import`."),
    c(
        "settings.import",
        "settings",
        "Apply a settings profile, all or none",
        "settings:j@0 reset_missing:b?",
    )
    .since("1.7")
    .result("as `settings.set_many`")
    .doc("`settings` is the object `settings.export` returned (`values` is accepted as a synonym). Validated and applied like `settings.set_many`. With `reset_missing: true` the settings that differ from the defaults and are not in the profile go back to their defaults, so the result is exactly the profile."),
    c(
        "hardware.profile",
        "settings",
        "The detected hardware profile and the recommended defaults (local, no identifiers)",
        "redetect:b?",
    )
    .since("1.2")
    .result("{ probing, stored, allow_in_reports, profile, recommendations }")
    .doc("`profile` is the versioned `opts-hardware` document (every fact is `{known: v}` or `{unknown: reason}`; `null` until the first probe finished). `redetect: true` starts a new probe in the background; poll again. Nothing is stored by this call. See the hardware probe charter."),
    c(
        "l10n.languages",
        "l10n",
        "List the languages (built in and installed packs) with completeness, authors and problems",
        "",
    )
    .since("1.1")
    .result("`{current, packs_dir, languages:[{code, name, native_name, direction, builtin, from_pack, authors, version, total, translated, copied, missing, percent, errors, warnings, outdated, missing_fonts, selectable, current, dir}], problems:[{folder, issues}]}`")
    .doc("`translated` counts messages that are really translated, `copied` those that are still the English text, `missing` those that show English. `selectable` is false for right-to-left packs. `problems` lists pack folders that cannot be used (no valid `pack.json`). Packs are files in `<data dir>/lang/<code>/`, see the localisation packs page."),
    c(
        "l10n.set",
        "l10n",
        "Switch the interface language (saved like a setting)",
        "code:s@0",
    )
    .since("1.1")
    .result("`{current, previous}`")
    .doc("`1004` for a language that is not installed, `1007` for a right-to-left pack (not supported yet). Stores `interface.language` and updates every text at once."),
    c(
        "l10n.reload",
        "l10n",
        "Scan the language folder again (new or edited packs)",
        "",
    )
    .since("1.1")
    .result("the `l10n.languages` result plus `{usable, broken, errors}`")
    .doc("The same as the *Reload languages* button. A pack that cannot be used or a file with errors never fails the call: read `problems` and `l10n.report`. If the active language is gone the game shows English."),
];
