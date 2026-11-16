//! German is translated, not copied: no wordy message is identical in both languages.

use super::*;

/// Messages that are the same in both languages on purpose (names, symbols, loanwords). Every
/// other identical pair is an untranslated string. Keep the list short and justified.
const SAME_IN_BOTH: &[&str] = &[];

#[test]
fn german_is_translated_not_copied() {
    let loc = whole_application();
    let mut same = Vec::new();
    for key in loc.keys_of("en") {
        let (en, de) = (loc.format_in("en", key), loc.format_in("de", key));
        // One word or less, or digits and symbols only: names and units are legitimately equal.
        let prose: String = {
            let mut depth = 0;
            en.chars()
                .filter(|c| match c {
                    '{' => {
                        depth += 1;
                        false
                    }
                    '}' => {
                        depth -= 1;
                        false
                    }
                    _ => depth == 0,
                })
                .collect()
        };
        let wordy = prose.split_whitespace().count() >= 2 && prose.chars().any(char::is_alphabetic);
        if wordy && en == de && !SAME_IN_BOTH.contains(&key) {
            same.push(format!("{key} = {en}"));
        }
    }
    assert!(
        same.is_empty(),
        "identical in English and German:\n{}",
        same.join("\n")
    );
}
