//! Item row entity spawning and decoration for the master list.

use bevy::prelude::*;

use crate::icons::icon_text;
use crate::interaction::focus::PointerOnly;
use crate::interaction::state::Selected;
use crate::interaction::tooltip::Tooltip;
use crate::l10n::{wizard_keys::md, L10nTooltip};
use crate::patterns::glyphs;
use crate::patterns::list_model::{ListEntry, Section, ITEM_HEIGHT};
use crate::patterns::master_detail::{MasterDetail, MasterRowButton, MasterStarButton, RowThumb};
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, ForegroundRole, RadiusToken, TypeStyleToken};
use crate::ui_id::{slug, UiId};
use crate::ui_tree::{A11yHidden, ListRow};

use super::{section_key, ThumbGlyph, BAR_GUTTER};

pub(super) struct ItemContext<'a> {
    pub root: Entity,
    pub prefix: &'a str,
    pub model: &'a MasterDetail,
    pub entry: &'a ListEntry,
    pub row: usize,
    pub section: Section,
    pub top: f32,
    pub max_chars: usize,
}

pub(super) fn spawn_item(commands: &mut Commands, ctx: &ItemContext<'_>) -> Entity {
    let selected = ctx.model.selected.as_deref() == Some(ctx.entry.id.as_str());
    let favourite = ctx.model.favourites.contains(&ctx.entry.id);
    let key = slug(&ctx.entry.id);
    let wrap = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(BAR_GUTTER),
                top: Val::Px(ctx.top),
                height: Val::Px(ITEM_HEIGHT),
                align_items: AlignItems::Center,
                column_gap: Val::Px(4.0),
                padding: UiRect::horizontal(Val::Px(4.0)),
                ..default()
            },
            Name::new("MasterRow"),
        ))
        .id();
    let button = spawn_row_button(commands, ctx, selected, &key);
    commands.entity(wrap).add_child(button);
    if !ctx.model.no_stars {
        let star = spawn_star(
            commands,
            ctx.root,
            ctx.prefix,
            ctx.entry,
            ctx.section,
            favourite,
            &key,
        );
        commands.entity(wrap).add_child(star);
    }
    wrap
}

fn spawn_row_button(
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

/// `text` cut to `max` characters with an ellipsis.
pub fn ellipsize(text: &str, max: usize) -> String {
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
