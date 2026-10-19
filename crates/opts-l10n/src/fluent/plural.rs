//! CLDR plural categories of the supported languages.

use super::*;

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
        "fr" => {
            if i <= 1 && whole || n > 0.0 && n < 2.0 && !whole {
                "one"
            } else if whole && i != 0 && i % 1_000_000 == 0 {
                "many"
            } else {
                "other"
            }
        }
        "pt" => {
            if (0.0..2.0).contains(&n) && (whole || n > 0.0) {
                "one"
            } else if whole && i != 0 && i % 1_000_000 == 0 {
                "many"
            } else {
                "other"
            }
        }
        "es" | "it" | "ca" => {
            if exactly_one {
                "one"
            } else if whole && i != 0 && i % 1_000_000 == 0 {
                "many"
            } else {
                "other"
            }
        }
        "ru" | "uk" | "be" if whole => {
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "hr" | "sr" | "bs" | "sh" if whole => {
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "pl" if whole => {
            if i == 1 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "cs" | "sk" if whole => match i {
            1 => "one",
            2..=4 => "few",
            _ => "other",
        },
        "lt" if whole => {
            if m10 == 1 && !(11..=19).contains(&m100) {
                "one"
            } else if (2..=9).contains(&m10) && !(11..=19).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "sl" if whole => match m100 {
            1 => "one",
            2 => "two",
            3 | 4 => "few",
            _ => "other",
        },
        "ar" if whole => match i {
            0 => "zero",
            1 => "one",
            2 => "two",
            _ if (3..=10).contains(&m100) => "few",
            _ if (11..=99).contains(&m100) => "many",
            _ => "other",
        },
        "he" if whole => match i {
            1 => "one",
            2 => "two",
            _ => "other",
        },
        "ro" | "mo" if whole => {
            if i == 1 {
                "one"
            } else if i == 0 || (1..=19).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "lv" if whole => {
            if i == 0 || (11..=19).contains(&m100) {
                "zero"
            } else if m10 == 1 && m100 != 11 {
                "one"
            } else {
                "other"
            }
        }
        _ => {
            if exactly_one {
                "one"
            } else {
                "other"
            }
        }
    }
}
