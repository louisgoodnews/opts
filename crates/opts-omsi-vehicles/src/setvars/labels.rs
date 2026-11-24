//! The label and the description of an option: its documented meaning, or its readable name.

use super::curated::CuratedSetvar;

/// Longest meaning text that is used as the label itself.
const LABEL_MAX_CHARS: usize = 36;

/// The label and the description: a short documented meaning is the label itself, a long one the
/// description under the readable name.
pub(super) fn labels(name: &str, doc: Option<&CuratedSetvar>) -> (String, String) {
    let meaning = doc.map(|d| d.meaning.trim()).unwrap_or_default();
    if meaning.is_empty() {
        return (humanise(name), String::new());
    }
    let short = meaning.chars().count() <= LABEL_MAX_CHARS && !meaning.contains([';', '(']);
    if short {
        (capitalise(meaning), String::new())
    } else {
        (humanise(name), capitalise(meaning))
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// The option's own name made readable (`vis_front_light` becomes `Vis front light`).
#[must_use]
pub fn humanise(name: &str) -> String {
    capitalise(name.replace(['_', '-'], " ").trim())
}
