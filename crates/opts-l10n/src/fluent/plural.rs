//! CLDR plural categories of the supported languages.

/// The CLDR cardinal plural categories `lang` uses, in the order a template lists them. Every
/// language has `other`. Languages without a rule here use `one` and `other`.
#[must_use]
pub fn plural_categories(lang: &str) -> &'static [&'static str] {
    match lang {
        "ja" | "zh" | "ko" | "vi" | "th" | "id" | "ms" | "lo" | "my" | "km" => &["other"],
        "ru" | "uk" | "be" | "hr" | "sr" | "bs" | "sh" => &["one", "few", "many", "other"],
        "pl" => &["one", "few", "many", "other"],
        "cs" | "sk" => &["one", "few", "many", "other"],
        "lt" => &["one", "few", "many", "other"],
        "sl" => &["one", "two", "few", "other"],
        "ar" => &["zero", "one", "two", "few", "many", "other"],
        "he" => &["one", "two", "other"],
        "ro" | "mo" => &["one", "few", "other"],
        "lv" => &["zero", "one", "other"],
        "ga" => &["one", "two", "few", "many", "other"],
        "cy" => &["zero", "one", "two", "few", "many", "other"],
        "fr" | "pt" | "es" | "it" | "ca" => &["one", "many", "other"],
        _ => &["one", "other"],
    }
}

mod other;
mod romance;
mod slavic;

use other::other_family_plural;
use romance::romance_plural;
use slavic::slavic_plural;

/// The plural category of `n` in `lang` (CLDR cardinal rules for whole numbers of the languages
/// in [`plural_categories`]; other languages: `one` for exactly 1, else `other`).
#[must_use]
pub fn plural_category(lang: &str, n: f64) -> &'static str {
    let lang = lang.split(['-', '_']).next().unwrap_or(lang);
    let whole = n.fract() == 0.0 && n >= 0.0;
    let i = if whole { n as u64 } else { 0 };
    let (m10, m100) = (i % 10, i % 100);
    let exactly_one = (n - 1.0).abs() < f64::EPSILON;

    match lang {
        "ja" | "zh" | "ko" | "vi" | "th" | "id" | "ms" | "lo" | "my" | "km" => "other",
        "fr" | "pt" | "es" | "it" | "ca" => romance_plural(lang, n, i, whole, exactly_one),
        "ru" | "uk" | "be" | "hr" | "sr" | "bs" | "sh" | "pl" | "cs" | "sk" | "sl" if whole => {
            slavic_plural(lang, i, m10, m100)
        }
        "lt" | "ar" | "he" | "ro" | "mo" | "lv" if whole => other_family_plural(lang, i, m10, m100),
        _ if exactly_one => "one",
        _ => "other",
    }
}
