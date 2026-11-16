//! The row button of a master list item and its selection marker and thumbnail slot.

use bevy::prelude::*;

use crate::interaction::focus::PointerOnly;
use crate::interaction::state::Selected;
use crate::interaction::tooltip::Tooltip;
use crate::patterns::master_detail::{MasterRowButton, RowThumb};
use crate::theme::{color::ColorRole, shape::RadiusLevel};
use crate::tokens::components::{BackgroundRole, ForegroundRole, RadiusToken};
use crate::ui_id::UiId;
use crate::ui_tree::{A11yHidden, ListRow};

use super::super::{section_key, ThumbGlyph};
use super::text::spawn_texts;
use super::ItemContext;
use crate::icons::icon_text;
use crate::patterns::list_model::ListEntry;

pub(super) fn spawn_row_button(
    commands: &mut Commands,
    ctx: &ItemContext<'_>,
    selected: bool,
    key: &str,
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
            root: ctx.root,
            row: ctx.row,
            id: ctx.entry.id.clone(),
        },
        UiId::new(format!("{}.{}.{key}", ctx.prefix, section_key(ctx.section))),
    ));
    if ctx.entry.title.chars().count() > ctx.max_chars {
        // Cut titles say their full text on hover and focus.
        button.insert(Tooltip::new(ctx.entry.title.clone()));
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
    let thumb = spawn_thumb(commands, ctx.root, ctx.entry);
    let texts = spawn_texts(commands, ctx.entry, fg, ctx.max_chars);
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
