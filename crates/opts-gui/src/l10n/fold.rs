//! Text folding for search: case- and diacritics-insensitive matching (menus, lists).

/// Lower-case text without diacritics, for matching: `Süd` and `sud`, `Café` and `cafe`, `ß` and
/// `ss` compare equal.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{0300}'..='\u{036f}' => {}
            'ß' => out.push_str("ss"),
            'æ' | 'Æ' => out.push_str("ae"),
            'œ' | 'Œ' => out.push_str("oe"),
            _ => {
                for l in c.to_lowercase() {
                    out.push(base_letter(l));
                }
            }
        }
    }
    out
}

/// The unaccented base of a precomposed Latin letter (other characters stay).
fn base_letter(c: char) -> char {
    const TABLE: &[(&str, char)] = &[
        ("àáâãäåāăą", 'a'),
        ("çćĉċč", 'c'),
        ("ďđ", 'd'),
        ("èéêëēĕėęě", 'e'),
        ("ĝğġģ", 'g'),
        ("ĥħ", 'h'),
        ("ìíîïĩīĭįı", 'i'),
        ("ĵ", 'j'),
        ("ķ", 'k'),
        ("ĺļľŀł", 'l'),
        ("ñńņňŉ", 'n'),
        ("òóôõöøōŏő", 'o'),
        ("ŕŗř", 'r'),
        ("śŝşš", 's'),
        ("ţťŧ", 't'),
        ("ùúûüũūŭůűų", 'u'),
        ("ŵ", 'w'),
        ("ýÿŷ", 'y'),
        ("źżž", 'z'),
    ];
    TABLE
        .iter()
        .find(|(set, _)| set.contains(c))
        .map_or(c, |(_, base)| *base)
}

/// Whether `option` matches `query`: every whitespace-separated word of the query occurs in it
/// (case- and diacritics-insensitive; punctuation may be left out, so `ubhf` finds `U-Bhf.`). An
/// empty query matches everything.
pub fn matches_query(option: &str, query: &str) -> bool {
    let hay = fold(option);
    let squeezed: String = hay.chars().filter(|c| c.is_alphanumeric()).collect();
    fold(query).split_whitespace().all(|word| {
        let w: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        hay.contains(word) || (!w.is_empty() && squeezed.contains(&w))
    })
}
