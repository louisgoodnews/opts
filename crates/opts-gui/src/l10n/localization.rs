//! The active localisation: the shipped and pack tables, the fallback chain and the lookups.

use std::collections::HashMap;

use bevy::prelude::*;

use super::fluent::{self, Table};
use super::report::LoadedPack;
use super::{pseudo, LOCALE_ENV};

/// The active locale and all message tables.
#[derive(Resource, Debug, Clone)]
pub struct Localization {
    pub(super) locale: String,
    pub(super) bundles: HashMap<String, Table>,
    /// Messages of language packs from the user directory: looked up before the built-in table
    /// of the same tag, so a pack overrides or extends it. Replaced as a whole on reload.
    pub(super) overlay: HashMap<String, Table>,
    /// What the last pack scan found.
    pub(super) packs: Vec<LoadedPack>,
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
