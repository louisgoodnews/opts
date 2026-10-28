//! Spawning of the row entities of the master list (only the visible window exists).

use bevy::prelude::*;

use crate::l10n::{wizard_keys::md, L10nKey};
use crate::patterns::list_model::{Row, RowLayout, Section};
use crate::patterns::master_detail::MasterDetail;
use crate::theme::{color::ColorRole, typography::TypeScale};
use crate::tokens::components::{ForegroundRole, TypeStyleToken};

mod item;
#[cfg(test)]
pub(super) use item::ellipsize;
use item::*;

/// The glyph in a thumbnail slot; the screen removes it when it puts an image there.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThumbGlyph;

/// Width kept free at the right of a row for the scroll bar.
pub(super) const BAR_GUTTER: f32 = 12.0;

pub(super) fn section_key(section: Section) -> &'static str {
    match section {
        Section::Recent => "recent",         // l10n-ok: id or variable name
        Section::Favourites => "favourites", // l10n-ok: id or variable name
        Section::All => "all",               // l10n-ok: id or variable name
        Section::Results => "results",       // l10n-ok: id or variable name
    }
}

fn header_key(section: Section) -> &'static str {
    match section {
        Section::Recent => md::SECTION_RECENT,
        Section::Favourites => md::SECTION_FAVOURITES,
        Section::All | Section::Results => md::SECTION_ALL,
    }
}

#[allow(clippy::too_many_arguments)]
/// Spawns the rows `range` as children of `content` and returns them.
pub(super) fn spawn_rows(
    commands: &mut Commands,
    root: Entity,
    prefix: &str,
    model: &MasterDetail,
    rows: &[Row],
    layout: &RowLayout,
    range: std::ops::Range<usize>,
    content: Entity,
    max_chars: usize,
) -> Vec<Entity> {
    let mut out = Vec::with_capacity(range.len());
    for i in range {
        let top = layout.top(i);
        let e = match rows[i] {
            Row::Header(section) => spawn_header(commands, section, top),
            Row::Item { entry, section } => spawn_item(
                commands,
                root,
                prefix,
                model,
                &model.entries[entry],
                i,
                section,
                top,
                max_chars,
            ),
        };
        out.push(e);
    }
    commands.entity(content).add_children(&out);
    out
}

fn spawn_header(commands: &mut Commands, section: Section, top: f32) -> Entity {
    let label = commands
        .spawn((
            Text::new(header_key(section)),
            L10nKey(header_key(section)),
            TextColor::default(),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::Label),
        ))
        .id();
    let header = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(top),
                height: Val::Px(crate::patterns::list_model::HEADER_HEIGHT),
                padding: UiRect::new(Val::Px(16.0), Val::Px(8.0), Val::Px(8.0), Val::Px(2.0)),
                align_items: AlignItems::End,
                ..default()
            },
            Name::new("MasterHeader"),
        ))
        .id();
    commands.entity(header).add_child(label);
    header
}

#[cfg(test)]
mod tests;
