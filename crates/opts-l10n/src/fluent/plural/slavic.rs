//! Plural rules of the Slavic family (Russian, Polish, Czech, Croatian, ...).

pub(super) fn slavic_plural(lang: &str, i: u64, m10: u64, m100: u64) -> &'static str {
    match lang {
        "ru" | "uk" | "be" => {
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "hr" | "sr" | "bs" | "sh" => {
            if m10 == 1 && m100 != 11 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "pl" => {
            if i == 1 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        "cs" | "sk" => match i {
            1 => "one",
            2..=4 => "few",
            _ => "other",
        },
        _ => match m100 {
            1 => "one",
            2 => "two",
            3 | 4 => "few",
            _ => "other",
        },
    }
}
