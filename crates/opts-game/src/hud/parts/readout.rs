//! One readout column of the instruments: its value and caption.

use super::*;

/// One readout column: value and caption.
pub(in crate::hud) fn readout(
    parent: &mut ChildSpawnerCommands,
    w: &'static HudWidget,
    value: (TypeScale, InstrumentInk),
    caption_key: Option<&'static str>,
    palette: &InstrumentPalette,
) {
    let width = match w.binding {
        Binding::Speed => ReadoutBox {
            normal: 92.0,
            compact: 66.0,
        },
        _ => ReadoutBox {
            normal: 56.0,
            compact: 42.0,
        },
    };
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // A fixed width keeps the value from jittering.
                min_width: Val::Px(width.normal),
                ..default()
            },
            width,
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
