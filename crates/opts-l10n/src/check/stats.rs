//! Completeness statistics and font script coverage checks for translation tables.

use super::*;

/// How complete a translation is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    /// Number of English messages.
    pub total: usize,
    /// English messages the table has (valid ones).
    pub present: usize,
    /// Present and really translated (not an untouched copy of the English prose).
    pub translated: usize,
    /// Present but identical to English prose: probably untouched scaffold text.
    pub copied: usize,
    /// English keys the table lacks (English shows instead).
    pub missing: Vec<String>,
    /// Keys the table has that English lacks (ignored).
    pub unknown: Vec<String>,
}

impl Stats {
    /// Percentage of English messages that are translated, 0 to 100.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            self.translated as f64 * 100.0 / self.total as f64
        }
    }
}

/// Completeness of `table` against `reference` (for language `lang`; English counts copies as
/// translated).
#[must_use]
pub fn stats(reference: &Reference, table: &Table, lang: &str) -> Stats {
    let mut s = Stats {
        total: reference.len(),
        ..Stats::default()
    };
    let english = language_of(lang) == "en";
    for key in reference.keys() {
        match table.messages.get(key) {
            None => s.missing.push(key.clone()),
            Some(p) => {
                s.present += 1;
                let same = reference.table.messages.get(key) == Some(p);
                if !english && same && is_prose(&visible_text(p)) {
                    s.copied += 1;
                } else {
                    s.translated += 1;
                }
            }
        }
    }
    let mut unknown: Vec<String> = table
        .messages
        .keys()
        .filter(|k| !reference.table.messages.contains_key(*k))
        .cloned()
        .collect();
    unknown.sort();
    s.unknown = unknown;
    s
}

/// Scripts a table's text needs that the bundled fonts (Noto Sans and DejaVu Sans: Latin, Greek,
/// Cyrillic) do not cover, and whether any is written right to left.
#[must_use]
pub fn scripts_beyond_fonts(table: &Table) -> (BTreeSet<&'static str>, bool) {
    let mut text = String::new();
    for p in table.messages.values() {
        collect_texts(p, &mut text);
    }
    let mut scripts = BTreeSet::new();
    let mut rtl = false;
    for c in text.chars() {
        let u = c as u32;
        let name = match u {
            0x0590..=0x05FF => Some(("Hebrew", true)),
            0x0600..=0x06FF | 0x0750..=0x077F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
                Some(("Arabic", true))
            }
            0x0700..=0x074F => Some(("Syriac", true)),
            0x0530..=0x058F => Some(("Armenian", false)),
            0x0900..=0x097F => Some(("Devanagari", false)),
            0x0980..=0x09FF => Some(("Bengali", false)),
            0x0B80..=0x0BFF => Some(("Tamil", false)),
            0x0E00..=0x0E7F => Some(("Thai", false)),
            0x10A0..=0x10FF => Some(("Georgian", false)),
            0x1100..=0x11FF | 0xAC00..=0xD7AF => Some(("Hangul", false)),
            0x1200..=0x137F => Some(("Ethiopic", false)),
            0x1780..=0x17FF => Some(("Khmer", false)),
            0x3040..=0x30FF => Some(("Kana", false)),
            0x3400..=0x4DBF | 0x4E00..=0x9FFF => Some(("CJK ideographs", false)),
            _ => None,
        };
        if let Some((n, r)) = name {
            scripts.insert(n);
            rtl |= r;
        }
    }
    (scripts, rtl)
}
