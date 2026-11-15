//! Lookups of the active localisation: the fallback chain and the formatting of messages.

use super::fluent::{self, Arg};
use super::Localization;

impl Localization {
    /// The pattern of `key` in the own table of `tag`: the pack's first, then the built-in one.
    pub(super) fn pattern(&self, tag: &str, key: &str) -> Option<&fluent::Pattern> {
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
        self.format(key, args).unwrap_or_else(|| format!("⟦{key}⟧")) // l10n-ok: the missing-key marker
    }

    /// The message `key` from the own table of `locale` only (no fallback), without variables;
    /// `⟦key⟧` when that table lacks it. Tests use it to compare languages.
    pub fn format_in(&self, locale: &str, key: &str) -> String {
        self.format_in_at(locale, key, 0)
            .unwrap_or_else(|| format!("⟦{key}⟧")) // l10n-ok: the missing-key marker
    }

    fn format_in_at(&self, locale: &str, key: &str, depth: usize) -> Option<String> {
        let args: &[(String, Arg)] = &[];
        let pattern = self.pattern(locale, key)?;
        let lang = locale.split('-').next().unwrap_or("en");
        let lookup = |k: &str| self.format_in_at(locale, k, depth + 1);
        Some(fluent::format(pattern, lang, args, &lookup, depth))
    }
}
