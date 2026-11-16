//! Finding a preset by a user's text: its id, slug, localised name or a word of one.

use super::*;

impl WeatherCatalogue {
    /// The conditions of the preset behind `id`.
    #[must_use]
    pub fn resolve(&self, id: &str) -> WeatherConditions {
        WeatherConditions::from_file(&self.file_of(id), &self.clouds)
    }

    /// The localised name of an installed preset for a UI locale (`de`, `en`); `None` for the
    /// built-in presets (the UI has message keys for them).
    #[must_use]
    pub fn display_name(&self, id: &str, locale: &str) -> Option<String> {
        let canonical = self.canonical_id(id);
        self.installed(&canonical)
            .map(|p| p.name(omsi_language(locale)))
    }

    /// The localised description of an installed preset.
    #[must_use]
    pub fn description(&self, id: &str, locale: &str) -> Option<String> {
        let canonical = self.canonical_id(id);
        self.installed(&canonical)
            .map(|p| p.description(omsi_language(locale)))
            .filter(|d| !d.is_empty())
    }

    /// The listed id a user-typed name stands for: an id, its slug, a localised name or a word of
    /// one of them (`rain`, `snow`), case-insensitively. Old built-in ids work as well.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<String> {
        let n = name.trim().to_lowercase();
        if n.is_empty() {
            return None;
        }
        let ids = self.ids();
        let slug = |s: &str| {
            s.to_lowercase()
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '#' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_owned()
        };
        let texts = |id: &str| -> BTreeSet<String> {
            let mut set = BTreeSet::from([id.to_lowercase()]);
            if let Some(p) = self.installed(id) {
                set.extend(p.texts.values().map(|t| t.name.to_lowercase()));
                set.insert(p.weather.clean_name().to_lowercase());
            }
            set
        };
        let exact = ids.iter().find(|id| {
            texts(id).contains(&n) || slug(id) == slug(&n) || {
                let names = texts(id);
                names.iter().any(|t| slug(t) == slug(&n))
            }
        });
        if let Some(id) = exact {
            return Some(id.clone());
        }
        // An old built-in id, spelled out.
        if let Some(b) = BUILTIN_IDS
            .iter()
            .find(|b| b.to_lowercase() == n || slug(b) == slug(&n))
        {
            return Some(self.canonical_id(b));
        }
        ids.iter()
            .find(|id| {
                texts(id).iter().any(|t| {
                    t.split(|c: char| !c.is_alphanumeric())
                        .any(|w| w.starts_with(&n))
                })
            })
            .cloned()
            .or_else(|| {
                // A word of a built-in id (`rain`, `snow`, `fog`, `clear`).
                BUILTIN_IDS
                    .iter()
                    .find(|b| b.to_lowercase().split(' ').any(|w| w.starts_with(&n)))
                    .map(|b| self.canonical_id(b))
            })
    }
}

