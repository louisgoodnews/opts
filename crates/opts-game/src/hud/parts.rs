//! Markers and building blocks of the HUD: the components the update systems look for and the
//! helpers that make panels, texts and readouts from instrument tokens.

use bevy::prelude::*;
use opts_gui::prelude::*;
use opts_gui::theme::InstrumentPalette;

use super::keys;
use super::profile::HudProfile;
use super::widget::{Binding, HudWidget, Slot, VehicleKind};
use crate::in_game::instruments::{InstrumentFade, InstrumentInk, InstrumentPanel};

/// Marks the root of the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HudRoot;

/// Marks the panel of a slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSlot(pub Slot);

/// Marks the root node of a widget.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudWidgetNode {
    /// The widget's id.
    pub id: &'static str,
    /// Where it was built.
    pub slot: Slot,
}

/// Which part of a widget a text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The value.
    Value,
    /// The caption or unit.
    Caption,
}

/// A text the update systems write.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudText {
    /// What it shows.
    pub binding: Binding,
    /// Value or caption.
    pub part: Part,
}

/// The pill of the alert slot.
#[derive(Component, Debug, Clone, Copy)]
pub struct AlertPill;

/// The profile the running HUD was built from.
#[derive(Resource, Debug, Clone)]
pub struct ActiveProfile {
    /// The kind.
    pub kind: VehicleKind,
    /// The profile.
    pub profile: HudProfile,
}

pub(super) fn texts(
    text: &str,
    scale: TypeScale,
    ink: InstrumentInk,
    palette: &InstrumentPalette,
) -> impl Bundle {
    let color = match ink {
        InstrumentInk::Text => palette.text,
        InstrumentInk::TextDim => palette.text_dim,
        InstrumentInk::Needle => palette.needle,
        InstrumentInk::Ok => palette.ok,
        InstrumentInk::Warning => palette.warning,
        InstrumentInk::Alarm => palette.alarm,
    };
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(color),
        TypeStyleToken(scale),
        ink,
        InstrumentFade(1.0),
    )
}

pub(super) fn panel_node(padding: UiRect, direction: FlexDirection, gap: f32) -> Node {
    Node {
        padding,
        flex_direction: direction,
        column_gap: Val::Px(gap),
        row_gap: Val::Px(gap),
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Px(1.0)),
        ..default()
    }
}

pub(super) fn panel(node: Node, radius: RadiusLevel, palette: &InstrumentPalette) -> impl Bundle {
    (
        node,
        BorderColor::all(palette.panel_border),
        BackgroundColor(palette.panel),
        InstrumentPanel,
        InstrumentFade(1.0),
        RadiusToken(radius),
    )
}

/// One readout column: value and caption.
pub(super) fn readout(
    parent: &mut ChildSpawnerCommands,
    w: &'static HudWidget,
    value: (TypeScale, InstrumentInk),
    caption_key: Option<&'static str>,
    palette: &InstrumentPalette,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // A fixed width keeps the value from jittering.
                min_width: Val::Px(match w.binding {
                    Binding::Speed => 92.0,
                    _ => 56.0,
                }),
                ..default()
            },
            HudWidgetNode {
                id: w.id,
                slot: w.slot,
            },
            UiId::new(format!("hud.{}", w.id)),
        ))
        .with_children(|col| {
            col.spawn((
                texts("", value.0, value.1, palette),
                HudText {
                    binding: w.binding,
                    part: Part::Value,
                },
            ));
            let mut caption = col.spawn((
                texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                HudText {
                    binding: w.binding,
                    part: Part::Caption,
                },
            ));
            if let Some(key) = caption_key {
                caption.insert(opts_gui::l10n::L10nKey(key));
            }
        });
}

pub(super) fn build_instrument(
    parent: &mut ChildSpawnerCommands,
    w: &'static HudWidget,
    palette: &InstrumentPalette,
) {
    match w.binding {
        Binding::Speed => readout(
            parent,
            w,
            (TypeScale::NumericLarge, InstrumentInk::Needle),
            None,
            palette,
        ),
        Binding::Gear => readout(
            parent,
            w,
            (TypeScale::NumericMedium, InstrumentInk::Ok),
            Some(keys::CAPTION_GEAR),
            palette,
        ),
        Binding::Rpm => readout(
            parent,
            w,
            (TypeScale::NumericMedium, InstrumentInk::Text),
            Some(keys::CAPTION_RPM),
            palette,
        ),
        Binding::ParkingBrake => readout(
            parent,
            w,
            (TypeScale::BodyMedium, InstrumentInk::Ok),
            Some(keys::CAPTION_PARK),
            palette,
        ),
        _ => {}
    }
}

pub(super) fn text_widget(
    parent: &mut ChildSpawnerCommands,
    w: &'static HudWidget,
    scale: TypeScale,
    ink: InstrumentInk,
    palette: &InstrumentPalette,
) {
    parent.spawn((
        texts("", scale, ink, palette),
        HudText {
            binding: w.binding,
            part: Part::Value,
        },
        HudWidgetNode {
            id: w.id,
            slot: w.slot,
        },
        UiId::new(format!("hud.{}", w.id)),
    ));
}
