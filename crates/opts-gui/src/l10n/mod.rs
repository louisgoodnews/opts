//! Localisation: message keys, variables, plurals, `en` and `de`, a fallback chain and the
//! pseudo-locale `en-XA` for tests.
//!
//! Strings live in `en.ftl` and `de.ftl` (a Fluent subset, see [`fluent`]). Code refers to them by
//! key: a text entity carries [`L10nKey`] (plus optional [`L10nArgs`]) and a system keeps its
//! `Text` current, also when the locale changes. The fallback chain is the user's locale, its
//! language without region, then English; a key missing everywhere shows as `⟦key⟧` so it is
//! noticed. A test forbids user-visible string literals in `opts-gui` outside this module.

use std::collections::HashMap;

use bevy::prelude::*;

pub mod fluent;
pub mod keys;
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
        }
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

    /// Locales with a table.
    pub fn available(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.bundles.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
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
            if let Some(pattern) = self.bundles.get(tag).and_then(|b| b.messages.get(key)) {
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

    /// Keys of the English table (the source of truth).
    pub fn english_keys(&self) -> Vec<&str> {
        self.bundles
            .get("en")
            .map(|b| b.messages.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Whether `locale` has its own table with `key`.
    pub fn has(&self, locale: &str, key: &str) -> bool {
        self.bundles
            .get(locale)
            .is_some_and(|b| b.messages.contains_key(key))
    }
}

/// A table parsed from `base` with the messages of `extra` added (the wizard's own file: three
/// parallel branches each own one table file instead of sharing one).
fn merged(base: &str, extra: &str) -> Table {
    let mut table = fluent::parse(base);
    let more = fluent::parse(extra);
    table.errors.extend(more.errors);
    table.messages.extend(more.messages);
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
