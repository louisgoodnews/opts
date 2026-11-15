//! Rail entries: the shared node builders (icon, label, red marker) of a navigation item.

use super::*;

pub(super) fn text_node(key: &'static str, role: ColorRole, scale: TypeScale) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

pub(super) fn icon_node(glyph: &'static str) -> impl Bundle {
    (
        Text::new(glyph),
        TextFont {
            font: FontSource::Handle(opts_gui::fonts::ICON_FONT),
            font_size: FontSize::Px(24.0),
            ..default()
        },
        TextColor::default(),
        ForegroundRole(ColorRole::OnSurfaceVariant),
        RailIcon,
        opts_gui::ui_tree::A11yHidden,
    )
}

pub(super) fn item_node() -> Node {
    Node {
        min_height: Val::Px(48.0),
        // never squeezed below its content when the rail is taller than the window
        flex_shrink: 0.0,
        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
        flex_wrap: FlexWrap::Wrap,
        align_content: AlignContent::Center,
        column_gap: Val::Px(12.0),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// Spawns one entry (icon, label, hidden red marker) as a child of `rail`.
pub(super) fn spawn_entry(
    rail: &mut ChildSpawnerCommands,
    icon: &'static str,
    title_key: &'static str,
    ui_id: String,
) -> Entity {
    let mut entry = rail.spawn((
        Button,
        item_node(),
        BackgroundColor(Color::NONE),
        StateLayerRole(ColorRole::OnSurface),
        RadiusToken(RadiusLevel::Full),
        Tooltip::new(""),
        L10nTooltip(title_key),
        UiId::new(ui_id),
    ));
    entry.with_children(|item| {
        item.spawn(icon_node(icon));
        item.spawn((
            text_node(title_key, ColorRole::OnSurface, TypeScale::BodyMedium),
            RailWideOnly,
            Node {
                flex_grow: 1.0,
                ..default()
            },
        ));
        // A direct child of the entry, so it sits above the disabled scrim (z 1) and stays red.
        item.spawn((
            Text::new(opts_gui::not_yet::MESSAGE),
            L10nKey(opts_gui::l10n::keys::NOT_AVAILABLE),
            TextColor::default(),
            ForegroundRole(ColorRole::Error),
            TypeStyleToken(TypeScale::Label),
            ZIndex(2),
            RailNotYet,
            opts_gui::ui_tree::A11yHidden,
            Node {
                display: Display::None,
                flex_basis: Val::Percent(100.0),
                margin: UiRect::left(Val::Px(36.0)),
                ..default()
            },
        ));
    });
    entry.id()
}

