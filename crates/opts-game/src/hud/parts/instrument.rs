//! Building an instrument panel and a text widget from their tokens.

use super::*;

pub(in crate::hud) fn build_instrument(
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
        Binding::RailDirection => readout(
            parent,
            w,
            (TypeScale::BodyMedium, InstrumentInk::Ok),
            Some(keys::CAPTION_DIRECTION),
            palette,
        ),
        Binding::RailLever => readout(
            parent,
            w,
            (TypeScale::NumericMedium, InstrumentInk::Text),
            Some(keys::CAPTION_LEVER),
            palette,
        ),
        Binding::Fuel => readout(
            parent,
            w,
            (TypeScale::NumericMedium, InstrumentInk::Text),
            Some(keys::CAPTION_FUEL),
            palette,
        ),
        Binding::FuelLow => readout(
            parent,
            w,
            (TypeScale::BodyMedium, InstrumentInk::Ok),
            Some(keys::CAPTION_RESERVE),
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

pub(in crate::hud) fn text_widget(
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
