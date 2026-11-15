//! Flattening of the model into rows: sections for an empty query, one flat result list otherwise.

use super::{ListEntry, Row, Section};

/// Flattens the model into rows. With no query and no chip the list has a *Recent* and a
/// *Favourites* section (when they have entries) before *All*; otherwise it is one flat result list.
pub fn build_rows(
    entries: &[ListEntry],
    recents: &[String],
    favourites: &[String],
    text: &str,
    chips: &[String],
) -> Vec<Row> {
    let filtering = !text.trim().is_empty() || !chips.is_empty();
    let mut rows = Vec::new();
    if filtering {
        for (i, e) in entries.iter().enumerate() {
            if e.matches(text, chips) {
                rows.push(Row::Item {
                    entry: i,
                    section: Section::Results,
                });
            }
        }
        return rows;
    }
    let recent: Vec<usize> = recents
        .iter()
        .filter_map(|id| entries.iter().position(|e| &e.id == id))
        .collect();
    let starred: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| favourites.contains(&e.id))
        .map(|(i, _)| i)
        .collect();
    let sectioned = !recent.is_empty() || !starred.is_empty();
    if !recent.is_empty() {
        rows.push(Row::Header(Section::Recent));
        rows.extend(recent.into_iter().map(|entry| Row::Item {
            entry,
            section: Section::Recent,
        }));
    }
    if !starred.is_empty() {
        rows.push(Row::Header(Section::Favourites));
        rows.extend(starred.into_iter().map(|entry| Row::Item {
            entry,
            section: Section::Favourites,
        }));
    }
    if sectioned {
        rows.push(Row::Header(Section::All));
    }
    rows.extend((0..entries.len()).map(|entry| Row::Item {
        entry,
        section: Section::All,
    }));
    rows
}
