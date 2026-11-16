//! The panel and text builders: texts, panel nodes and the panel bundle.

use super::*;

pub(in crate::hud) fn texts(
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

pub(in crate::hud) fn panel_node(padding: UiRect, direction: FlexDirection, gap: f32) -> Node {
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

pub(in crate::hud) fn panel(
    node: Node,
    radius: RadiusLevel,
    palette: &InstrumentPalette,
) -> impl Bundle {
    (
        node,
        BorderColor::all(palette.panel_border),
        BackgroundColor(palette.panel),
        InstrumentPanel,
        InstrumentFade(1.0),
        RadiusToken(radius),
    )
}
