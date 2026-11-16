//! The scan chip's components and its spawn: the frame, the icon, the label and the reason caption.

use bevy::prelude::*;
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

use crate::content_facts::WantsContentFacts;

use super::labels::look;
use super::ScanLevel;

/// The chip (frame).
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChip;
/// The icon inside the chip.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipIcon;
/// The text inside the chip (level word and time of the last scan).
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipLabel;
/// The caption below the chip: the reason and what to do.
#[derive(Component, Debug, Clone, Copy)]
pub struct ScanChipReason;

/// Spawns the chip and its caption below the *Scan now* row; [`refresh_scan_chip`] fills it.
pub fn spawn_chip(c: &mut Commands, pane: Entity) {
    let (bg, fg, glyph) = look(ScanLevel::Needed);
    let wrap = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(8.0)),
                ..default()
            },
            ChildOf(pane),
        ))
        .id();
    let chip = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundRole(bg),
            RadiusToken(RadiusLevel::Full),
            Tooltip::new(""),
            ScanChip,
            WantsContentFacts,
            UiId::new("settings.content.scan-status"),
            ChildOf(wrap),
        ))
        .id();
    c.spawn((
        Text::new(glyph),
        TextFont {
            font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor::default(),
        ForegroundRole(fg),
        opts_gui::ui_tree::A11yHidden,
        ScanChipIcon,
        ChildOf(chip),
    ));
    c.spawn((
        Text::new(""),
        L10nKey("scan-chip"),
        L10nArgs::default(),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(fg),
        ScanChipLabel,
        ChildOf(chip),
    ));
    c.spawn((
        Text::new(""),
        L10nKey("scan-why-never"),
        L10nArgs::default(),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        ScanChipReason,
        ChildOf(wrap),
    ));
}
