//! Spawning of the master notice: the empty-state text and the clear-filter button.

use bevy::prelude::*;

use super::container;
use crate::components::button::ButtonBuilder;
use crate::l10n::{wizard_keys::md, L10nArgs, L10nKey};
use crate::patterns::master_detail::{MasterClearButton, MasterDetailParts};
use crate::theme::{color::ColorRole, typography::TypeScale};
use crate::tokens::components::{ForegroundRole, TypeStyleToken};
use crate::ui_id::UiId;

pub(super) fn spawn_notice(commands: &mut Commands, p: MasterDetailParts, prefix: &str) {
    commands.entity(p.notice).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(12.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::None,
            ..container(FlexDirection::Column)
        },
        Name::new("MasterNotice"),
    ));
    commands.entity(p.notice_text).insert((
        Text::new(md::NO_RESULTS),
        L10nKey(md::NO_RESULTS),
        L10nArgs::one("query", ""), // l10n-ok: id or variable name
        TextColor::default(),
        TextLayout::new(Justify::Center, LineBreak::WordBoundary),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        TypeStyleToken(TypeScale::BodyMedium),
    ));
    let clear = ButtonBuilder::outlined(md::CLEAR_FILTER)
        .with_key(md::CLEAR_FILTER)
        .spawn(commands);
    commands.entity(clear).insert((
        MasterClearButton(p.root),
        UiId::new(format!("{prefix}.clear")),
    ));
    commands
        .entity(p.notice)
        .add_children(&[p.notice_text, clear]);
}
