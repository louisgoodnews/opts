//! Plural rules of the remaining families (Lithuanian, Arabic, Hebrew, Romanian, Latvian).

pub(super) fn other_family_plural(lang: &str, i: u64, m10: u64, m100: u64) -> &'static str {
    match lang {
        "lt" => {
            if m10 == 1 && !(11..=19).contains(&m100) {
                "one"
            } else if (2..=9).contains(&m10) && !(11..=19).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "ar" => match i {
            0 => "zero",
            1 => "one",
            2 => "two",
            _ if (3..=10).contains(&m100) => "few",
            _ if (11..=99).contains(&m100) => "many",
            _ => "other",
        },
        "he" => match i {
            1 => "one",
            2 => "two",
            _ => "other",
        },
        "ro" | "mo" => {
            if i == 1 {
                "one"
            } else if i == 0 || (1..=19).contains(&m100) {
                "few"
            } else {
                "other"
            }
        }
        "lv" => {
            if i == 0 || (11..=19).contains(&m100) {
                "zero"
            } else if m10 == 1 && m100 != 11 {
                "one"
            } else {
                "other"
            }
        }
        _ => "other",
    }
}
