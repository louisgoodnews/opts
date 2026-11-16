//! Plural rules of the Romance family (French, Portuguese and the rest).

pub(super) fn romance_plural(lang: &str, n: f64, i: u64, whole: bool, exactly_one: bool) -> &'static str {
    match lang {
        "fr" => {
            if i <= 1 && whole || n > 0.0 && n < 2.0 && !whole {
                "one"
            } else if whole && i != 0 && i.is_multiple_of(1_000_000) {
                "many"
            } else {
                "other"
            }
        }
        "pt" => {
            if (0.0..2.0).contains(&n) && (whole || n > 0.0) {
                "one"
            } else if whole && i != 0 && i.is_multiple_of(1_000_000) {
                "many"
            } else {
                "other"
            }
        }
        _ => {
            if exactly_one {
                "one"
            } else if whole && i != 0 && i.is_multiple_of(1_000_000) {
                "many"
            } else {
                "other"
            }
        }
    }
}
