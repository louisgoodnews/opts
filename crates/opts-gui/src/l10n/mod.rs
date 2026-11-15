//! Localisation: message keys, variables, plurals, `en` and `de`, a fallback chain and the
//! pseudo-locale `en-XA` for tests.
//!
//! Strings live in `en.ftl` and `de.ftl` (a Fluent subset, see [`fluent`]). Code refers to them by
//! key: a text entity carries [`L10nKey`] (plus optional [`L10nArgs`]) and a system keeps its
//! `Text` current, also when the locale changes. The fallback chain is the user's locale, its
//! language without region, then English; a key missing everywhere shows as `⟦key⟧` so it is
//! noticed. A test forbids user-visible string literals in `opts-gui` outside this module.

use bevy::prelude::*;

pub mod fold;
pub use opts_l10n::fluent;
pub mod keys;
pub mod packs;
pub mod pseudo;
pub mod wizard_keys;

mod localization;
mod lookup;
mod report;
mod system;

pub use localization::Localization;
pub use report::{LanguageInfo, LanguageReport};
pub use system::{build, key_of, localize_system, text};

use fluent::{Arg, Args};

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

#[cfg(test)]
mod tests;
