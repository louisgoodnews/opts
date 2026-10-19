//! Localisation: message keys, variables, plurals, `en` and `de`, a fallback chain and the
//! pseudo-locale `en-XA` for tests.
//!
//! Strings live in `en.ftl` and `de.ftl` (a Fluent subset, see [`fluent`]). Code refers to them by
//! key: a text entity carries [`L10nKey`] (plus optional [`L10nArgs`]) and a system keeps its
//! `Text` current, also when the locale changes. The fallback chain is the user's locale, its
//! language without region, then English; a key missing everywhere shows as `⟦key⟧` so it is
//! noticed. A test forbids user-visible string literals in `opts-gui` outside this module.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use opts_l10n::check::Issue;
use opts_l10n::pack::{Direction, Pack};

pub mod fold;
pub use opts_l10n::fluent;
pub mod keys;
pub mod packs;
pub mod pseudo;
pub mod wizard_keys;

use fluent::{Arg, Args, Table};

/// Environment variable that selects the locale (`en`, `de`, `en-XA`).
pub const LOCALE_ENV: &str = "OPTS_LOCALE";

/// The message key a text element was built from; makes ids and tests independent of the language.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct L10nKey(pub &'static str);

/// Variables of the message of an [`L10nKey`] element.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct L10nArgs(pub Args);

impl L10nArgs {
    /// One variable.
    pub fn one(name: &str, value: impl Into<Arg>) -> Self {
        Self(vec![(name.to_string(), value.into())])
    }
}

/// A tooltip taken from a message key (resolved into [`crate::interaction::tooltip::Tooltip`]).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct L10nTooltip(pub &'static str);

/// The active locale and all message tables.
#[derive(Resource, Debug, Clone)]
pub struct Localization {
    locale: String,
    bundles: HashMap<String, Table>,
    /// Messages of language packs from the user directory: looked up before the built-in table
    /// of the same tag, so a pack overrides or extends it. Replaced as a whole on reload.
    overlay: HashMap<String, Table>,
    /// What the last pack scan found.
    packs: Vec<LoadedPack>,
}

/// What the localisation keeps of a pack after loading it.
#[derive(Debug, Clone)]
struct LoadedPack {
    pack: Pack,
}

/// A language the interface can be shown in (a built-in one or an installed pack).
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageInfo {
    /// The tag (`de`, `fr`, `pt-BR`).
    pub code: String,
    /// English name.
    pub name: String,
    /// Name in the language itself.
    pub native_name: String,
    /// Writing direction.
    pub direction: Direction,
    /// `true` for `en` and `de` (shipped), `false` for a pack.
    pub builtin: bool,
    /// `true` when a pack changes or extends this language.
    pub from_pack: bool,
    /// Authors named by the pack (built-in languages: the project).
    pub authors: Vec<String>,
    /// Version of the pack.
    pub version: String,
    /// Number of English messages.
    pub total: usize,
    /// Messages really translated.
    pub translated: usize,
    /// Messages present but untouched copies of the English text.
    pub copied: usize,
    /// Messages that fall back to English (missing).
    pub missing: usize,
    /// Findings of the pack check.
    pub errors: usize,
    /// Warnings of the pack check.
    pub warnings: usize,
    /// The pack was made against an older English text.
    pub outdated: bool,
    /// Scripts used that the bundled fonts do not cover.
    pub missing_fonts: Vec<String>,
    /// The folder of the pack.
    pub dir: Option<PathBuf>,
    /// `false` for a right-to-left language (not supported yet).
    pub selectable: bool,
}

impl LanguageInfo {
    /// Percentage of English messages that are translated, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            self.translated as f64 * 100.0 / self.total as f64
        }
    }
}

/// The report of one language: completeness, the keys that fall back to English, and every
/// finding of the pack check.
#[derive(Debug, Clone)]
pub struct LanguageReport {
    /// The language.
    pub info: LanguageInfo,
    /// Keys that show English.
    pub missing_keys: Vec<String>,
    /// Keys present but identical to English prose.
    pub copied_keys: Vec<String>,
    /// Findings, `file:line` relative to the pack folder.
    pub issues: Vec<Issue>,
}

impl Default for Localization {
    fn default() -> Self {
        Self::new("en")
    }
}

impl Localization {
    /// Loads the shipped tables and selects `locale`.
    pub fn new(locale: &str) -> Self {
        let en = merged(include_str!("en.ftl"), include_str!("wizard_en.ftl"));
        let de = merged(include_str!("de.ftl"), include_str!("wizard_de.ftl"));
        let mut bundles = HashMap::new();
        bundles.insert(pseudo::PSEUDO_TAG.to_string(), pseudo::pseudo_table(&en));
        bundles.insert("en".to_string(), en);
        bundles.insert("de".to_string(), de);
        Self {
            locale: locale.to_string(),
            bundles,
            overlay: HashMap::new(),
            packs: Vec::new(),
        }
    }

    /// Adds the messages of `source` (Fluent subset) to the table of `locale`, so a crate can ship
    /// its own `.ftl` files next to its screens. Existing keys are replaced; the pseudo-locale
    /// follows the English table. Returns the parse problems (empty for a clean file).
    pub fn extend(&mut self, locale: &str, source: &str) -> Vec<String> {
        let parsed = fluent::parse(source);
        let errors = parsed.errors.clone();
        let table = self.bundles.entry(locale.to_string()).or_default();
        append_order(table, &parsed.order, &parsed.messages);
        table.messages.extend(parsed.messages);
        table.comments.extend(parsed.comments);
        if locale == "en" {
            if let Some(en) = self.bundles.get("en") {
                let pseudo = pseudo::pseudo_table(en);
                self.bundles.insert(pseudo::PSEUDO_TAG.to_string(), pseudo);
            }
        }
        errors
    }

    /// The locale from [`LOCALE_ENV`], else English.
    pub fn detect() -> Self {
        Self::new(&std::env::var(LOCALE_ENV).unwrap_or_else(|_| "en".into()))
    }

    /// The active locale tag.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Switches the locale; elements update on the next frame.
    pub fn set_locale(&mut self, locale: &str) {
        self.locale = locale.to_string();
    }

    /// Locales with a table (built-in or from a pack).
    pub fn available(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .bundles
            .keys()
            .chain(self.overlay.keys())
            .map(String::as_str)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The pattern of `key` in the own table of `tag`: the pack's first, then the built-in one.
    fn pattern(&self, tag: &str, key: &str) -> Option<&fluent::Pattern> {
        self.overlay
            .get(tag)
            .and_then(|t| t.messages.get(key))
            .or_else(|| self.bundles.get(tag).and_then(|t| t.messages.get(key)))
    }

    /// The tables to try, best first: the locale, its language, English.
    fn chain(&self) -> Vec<&str> {
        let mut chain = vec![self.locale.as_str()];
        if let Some((lang, _)) = self.locale.split_once('-') {
            chain.push(lang);
        }
        chain.push("en");
        chain.dedup();
        chain
    }

    /// The message `key`, or `None` when no table in the chain has it.
    pub fn format(&self, key: &str, args: &[(String, Arg)]) -> Option<String> {
        self.format_at(key, args, 0)
    }

    fn format_at(&self, key: &str, args: &[(String, Arg)], depth: usize) -> Option<String> {
        for tag in self.chain() {
            if let Some(pattern) = self.pattern(tag, key) {
                let lang = tag.split('-').next().unwrap_or("en");
                let lookup = |k: &str| self.format_at(k, args, depth + 1);
                return Some(fluent::format(pattern, lang, args, &lookup, depth));
            }
        }
        None
    }

    /// The message without variables.
    pub fn t(&self, key: &str) -> String {
        self.t_args(key, &[])
    }

    /// The message with variables; `⟦key⟧` when it is missing everywhere.
    pub fn t_args(&self, key: &str, args: &[(String, Arg)]) -> String {
        self.format(key, args).unwrap_or_else(|| format!("⟦{key}⟧"))
    }

    /// The message `key` from the own table of `locale` only (no fallback), without variables;
    /// `⟦key⟧` when that table lacks it. Tests use it to compare languages.
    pub fn format_in(&self, locale: &str, key: &str) -> String {
        self.format_in_at(locale, key, 0)
            .unwrap_or_else(|| format!("⟦{key}⟧"))
    }

    fn format_in_at(&self, locale: &str, key: &str, depth: usize) -> Option<String> {
        let args: &[(String, Arg)] = &[];
        let pattern = self.pattern(locale, key)?;
        let lang = locale.split('-').next().unwrap_or("en");
        let lookup = |k: &str| self.format_in_at(locale, k, depth + 1);
        Some(fluent::format(pattern, lang, args, &lookup, depth))
    }

    /// Keys of the English table (the source of truth).
    pub fn english_keys(&self) -> Vec<&str> {
        self.bundles
            .get("en")
            .map(|b| b.messages.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Keys of the own table of `locale` (no fallback), sorted.
    pub fn keys_of(&self, locale: &str) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .bundles
            .get(locale)
            .into_iter()
            .chain(self.overlay.get(locale))
            .flat_map(|b| b.messages.keys().map(String::as_str))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The variables (`$name`, selectors included) and message references the pattern of `key`
    /// in the own table of `locale` uses, sorted; `None` when the table lacks the key. German
    /// and English must agree on them, or a variable is lost in translation.
    pub fn variables_of(&self, locale: &str, key: &str) -> Option<Vec<String>> {
        let pattern = self.pattern(locale, key)?;
        Some(fluent::variables(pattern).into_iter().collect())
    }

    /// Whether `locale` has its own table with `key`.
    pub fn has(&self, locale: &str, key: &str) -> bool {
        self.pattern(locale, key).is_some()
    }
}

/// Adds the ids of a later file to the table's file order (each once).
fn append_order(table: &mut Table, order: &[String], new: &HashMap<String, fluent::Pattern>) {
    for k in order {
        if !table.messages.contains_key(k) && new.contains_key(k) {
            table.order.push(k.clone());
        }
    }
}

/// A table parsed from `base` with the messages of `extra` added (the wizard's own file: three
/// parallel branches each own one table file instead of sharing one).
fn merged(base: &str, extra: &str) -> Table {
    let mut table = fluent::parse(base);
    let more = fluent::parse(extra);
    table.errors.extend(more.errors);
    append_order(&mut table, &more.order, &more.messages);
    table.messages.extend(more.messages);
    table.comments.extend(more.comments);
    table
}

/// The key of an element or of one of its direct children (the label of a control). The
/// not-yet marker's own key does not name a control.
pub fn key_of(world: &World, entity: Entity) -> Option<String> {
    let e = world.get_entity(entity).ok()?;
    let own = |e: &EntityRef| {
        e.get::<L10nKey>()
            .filter(|k| k.0 != keys::NOT_AVAILABLE)
            .map(|k| k.0.to_string())
    };
    own(&e).or_else(|| {
        e.get::<Children>()
            .into_iter()
            .flat_map(|c| c.iter())
            .find_map(|child| world.get_entity(child).ok().and_then(|c| own(&c)))
    })
}

/// Query item: a text with a key.
type LocalizedText<'a> = (Ref<'a, L10nKey>, Option<Ref<'a, L10nArgs>>, &'a mut Text);
/// Query item: a tooltip with a key.
type LocalizedTip<'a> = (
    Ref<'a, L10nTooltip>,
    &'a mut crate::interaction::tooltip::Tooltip,
);
/// Query item: the reason of a not-yet control.
type LocalizedReason<'a> = (
    Ref<'a, crate::not_yet::NotAvailableYet>,
    &'a mut crate::interaction::state::DisabledReason,
);

/// Keeps the text of every [`L10nKey`] element, every [`L10nTooltip`] and every not-yet reason
/// current.
pub fn localize_system(
    loc: Res<Localization>,
    mut texts: Query<LocalizedText>,
    mut tips: Query<LocalizedTip>,
    mut reasons: Query<LocalizedReason>,
) {
    let all = loc.is_changed();
    for (key, args, mut text) in &mut texts {
        if all || key.is_changed() || args.as_ref().is_some_and(|a| a.is_changed()) {
            let a = args.as_ref().map_or(&[][..], |a| a.0.as_slice());
            let new = loc.t_args(key.0, a);
            if text.0 != new {
                text.0 = new;
            }
        }
    }
    for (key, mut tip) in &mut tips {
        if all || key.is_added() {
            tip.0 = loc.t(key.0).into();
        }
    }
    for (marker, mut reason) in &mut reasons {
        if all || marker.is_added() {
            let args: Args = vec![("milestone".into(), marker.place.milestone.into())];
            reason.0 = loc.t_args(keys::NOT_AVAILABLE_REASON, &args).into();
        }
    }
}

/// A `Text` bundle that shows the message `key` (resolved in the next frame).
pub fn text(key: &'static str) -> impl Bundle {
    (Text::new(key), L10nKey(key))
}

/// Registers the resource and the update system.
pub fn build(app: &mut App) {
    if !app.world().contains_resource::<Localization>() {
        app.insert_resource(Localization::detect());
    }
    app.add_systems(
        PostUpdate,
        localize_system.before(bevy::ui::UiSystems::Content),
    );
}

#[cfg(test)]
mod tests;
