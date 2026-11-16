//! Item row entity spawning and decoration for the master list.

use bevy::prelude::*;

use crate::icons::icon_text;
use crate::interaction::focus::PointerOnly;
use crate::interaction::tooltip::Tooltip;
use crate::l10n::{wizard_keys::md, L10nTooltip};
use crate::patterns::glyphs;
use crate::patterns::list_model::{ListEntry, Section, ITEM_HEIGHT};
use crate::patterns::master_detail::{MasterDetail, MasterStarButton};
use crate::theme::{color::ColorRole, shape::RadiusLevel};
use crate::tokens::components::{ForegroundRole, RadiusToken};
use crate::ui_id::{slug, UiId};
use crate::ui_tree::A11yHidden;

use super::{section_key, BAR_GUTTER};

mod row;
mod text;

use row::spawn_row_button;
#[cfg(test)]
pub use text::ellipsize;

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
