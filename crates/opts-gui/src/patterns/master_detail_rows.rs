//! Spawning of the row entities of the master list (only the visible window exists).

use bevy::prelude::*;

use crate::icons::icon_text;
use crate::interaction::focus::PointerOnly;
use crate::interaction::state::Selected;
use crate::interaction::tooltip::Tooltip;
use crate::l10n::{wizard_keys::md, L10nKey, L10nTooltip};
use crate::patterns::glyphs;
use crate::patterns::list_model::{ListEntry, Row, RowLayout, Section, ITEM_HEIGHT};
use crate::patterns::master_detail::{MasterDetail, MasterRowButton, MasterStarButton, RowThumb};
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, ForegroundRole, RadiusToken, TypeStyleToken};
use crate::ui_id::{slug, UiId};
use crate::ui_tree::{A11yHidden, ListRow};

/// The glyph in a thumbnail slot; the screen removes it when it puts an image there.
#[derive(Component, Debug, Clone, Copy)]
pub struct ThumbGlyph;

/// Width kept free at the right of a row for the scroll bar.
const BAR_GUTTER: f32 = 12.0;

fn section_key(section: Section) -> &'static str {
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

#[allow(clippy::too_many_arguments)]
fn spawn_item(
    commands: &mut Commands,
    root: Entity,
    prefix: &str,
    model: &MasterDetail,
    entry: &ListEntry,
    row: usize,
    section: Section,
    top: f32,
    max_chars: usize,
) -> Entity {
    let selected = model.selected.as_deref() == Some(entry.id.as_str());
    let favourite = model.favourites.contains(&entry.id);
    let key = slug(&entry.id);
    let wrap = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(BAR_GUTTER),
                top: Val::Px(top),
                height: Val::Px(ITEM_HEIGHT),
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                padding: UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            Name::new("MasterRow"),
        ))
        .id();
    let button = spawn_row_button(
        commands, root, prefix, entry, row, section, selected, &key, max_chars,
    );
    let star = spawn_star(commands, root, prefix, entry, section, favourite, &key);
    commands.entity(wrap).add_children(&[button, star]);
    wrap
}

#[allow(clippy::too_many_arguments)]
fn spawn_row_button(
    commands: &mut Commands,
    root: Entity,
    prefix: &str,
    entry: &ListEntry,
    row: usize,
    section: Section,
    selected: bool,
    key: &str,
    max_chars: usize,
) -> Entity {
    let mut button = commands.spawn((
        crate::components::button::Button,
        PointerOnly,
        ListRow,
        Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            min_width: Val::Px(0.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        RadiusToken(RadiusLevel::Medium),
        MasterRowButton {
            root,
            row,
            id: entry.id.clone(),
        },
        UiId::new(format!("{prefix}.{}.{key}", section_key(section))),
    ));
    if entry.title.chars().count() > max_chars {
        // Cut titles say their full text on hover and focus.
        button.insert(Tooltip::new(entry.title.clone()));
    }
    if selected {
        button.insert((Selected, BackgroundRole(ColorRole::SecondaryContainer)));
    }
    let button = button.id();
    let fg = if selected {
        ColorRole::OnSecondaryContainer
    } else {
        ColorRole::OnSurface
    };
    let marker = commands
        .spawn((
            Node {
                width: Val::Px(4.0),
                height: Val::Px(32.0),
                flex_shrink: 0.0,
                ..default()
            },
            RadiusToken(RadiusLevel::Full),
            Name::new("SelectionMarker"),
        ))
        .id();
    if selected {
        commands
            .entity(marker)
            .insert(BackgroundRole(ColorRole::Primary));
    }
    let thumb = spawn_thumb(commands, root, entry);
    let texts = spawn_texts(commands, entry, fg, max_chars);
    commands
        .entity(button)
        .add_children(&[marker, thumb, texts]);
    button
}

fn spawn_thumb(commands: &mut Commands, root: Entity, entry: &ListEntry) -> Entity {
    let mut slot = commands.spawn((
        Node {
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundRole(ColorRole::SurfaceContainerHigh),
        RadiusToken(RadiusLevel::Small),
        Name::new("RowThumbSlot"),
    ));
    if let Some(key) = &entry.thumb {
        slot.insert(RowThumb {
            root,
            key: key.clone(),
        });
    }
    let slot = slot.id();
    if let Some(glyph) = entry.glyph {
        let g = commands
            .spawn((
                icon_text(glyph, 22.0, TextColor::default().0),
                ForegroundRole(ColorRole::OnSurfaceVariant),
                A11yHidden,
                ThumbGlyph,
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(g);
    }
    slot
}

/// `text` cut to `max` characters with an ellipsis.
fn ellipsize(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn spawn_texts(
    commands: &mut Commands,
    entry: &ListEntry,
    fg: ColorRole,
    max_chars: usize,
) -> Entity {
    let col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Name::new("RowTexts"),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(ellipsize(&entry.title, max_chars)),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor::default(),
            ForegroundRole(fg),
            TypeStyleToken(TypeScale::TitleMedium),
        ))
        .id();
    commands.entity(col).add_child(title);
    if !entry.subtitle.is_empty() {
        let sub = commands
            .spawn((
                Text::new(ellipsize(&entry.subtitle, max_chars + 6)),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor::default(),
                ForegroundRole(if fg == ColorRole::OnSurface {
                    ColorRole::OnSurfaceVariant
                } else {
                    fg
                }),
                TypeStyleToken(TypeScale::Label),
            ))
            .id();
        commands.entity(col).add_child(sub);
    }
    col
}

fn spawn_star(
    commands: &mut Commands,
    root: Entity,
    prefix: &str,
    entry: &ListEntry,
    section: Section,
    favourite: bool,
    key: &str,
) -> Entity {
    let star = commands
        .spawn((
            crate::components::button::Button,
            PointerOnly,
            Node {
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            RadiusToken(RadiusLevel::Full),
            MasterStarButton {
                root,
                id: entry.id.clone(),
            },
            Tooltip::new(""),
            L10nTooltip(if favourite {
                md::STAR_REMOVE
            } else {
                md::STAR_ADD
            }),
            UiId::new(format!("{prefix}.star.{}.{key}", section_key(section))),
        ))
        .id();
    let glyph = commands
        .spawn((
            icon_text(
                if favourite {
                    glyphs::STAR
                } else {
                    glyphs::STAR_BORDER
                },
                22.0,
                TextColor::default().0,
            ),
            ForegroundRole(if favourite {
                ColorRole::Primary
            } else {
                ColorRole::OnSurfaceVariant
            }),
            A11yHidden,
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(star).add_child(glyph);
    star
}

#[cfg(test)]
mod tests {
    use super::ellipsize;

    #[test]
    fn long_titles_are_cut_with_an_ellipsis_and_short_ones_are_kept() {
        assert_eq!(ellipsize("Berlin", 20), "Berlin");
        assert_eq!(ellipsize("Aachener Stadtbus Variante", 10), "Aachener …");
        assert_eq!(ellipsize("äöü", 3), "äöü");
    }
}
