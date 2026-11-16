//! The pseudo-locale covers every key and lengthens every message by at least forty percent.

use super::*;
use opts_gui::l10n::pseudo::PSEUDO_TAG;

#[test]
fn the_pseudo_locale_covers_everything_and_lengthens_by_at_least_forty_percent() {
    let loc = whole_application();
    assert!(loc.available().contains(&PSEUDO_TAG));
    let mut short = Vec::new();
    for key in loc.keys_of("en") {
        assert!(loc.has(PSEUDO_TAG, key), "pseudo lacks `{key}`");
        let (en, xa) = (loc.format_in("en", key), loc.format_in(PSEUDO_TAG, key));
        // Messages with a plural selector pad each variant; compare the rendered text.
        let visible = |t: &str| {
            let mut n = 0;
            let mut depth = 0;
            for c in t.chars() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ if depth == 0 => n += 1,
                    _ => {}
                }
            }
            n
        };
        let (a, b) = (visible(&en), visible(&xa));
        // brackets and padding: [ text ~~~ ]
        if a > 0 && (b as f32) < a as f32 * 1.4 {
            short.push(format!("{key}: {a} -> {b}"));
        }
    }
    assert!(
        short.is_empty(),
        "pseudo-locale too short:\n{}",
        short.join("\n")
    );
}
