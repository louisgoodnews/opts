//! The configuration page: the vehicle options and the marker of options that need a new build.

use super::*;

pub(in crate::in_game::vehicle_menu) fn build_config(
    commands: &mut Commands,
    _theme: &MaterialTheme,
    menu: &VehicleMenu,
    v: &VehicleItem,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 70.0)
        .with_width_range(480.0, 720.0)
        .spawn(commands);
    commands.entity(root).insert(MenuRoot);
    commands.entity(root).insert(GlobalZIndex(1100));
    let items = get_vehicle_setvars(v, &menu.setvars, &menu.repaint);
    commands.entity(content).with_children(|p| {
        label(p, "Vehicle configuration", 20.0, ColorRole::Primary);
        label(p, v.full_name.clone(), 12.0, ColorRole::OnSurfaceVariant);
        if items.is_empty() {
            label(
                p,
                "This vehicle declares no configurable equipment options.",
                13.0,
                ColorRole::OnSurface,
            );
        }
        scroll_list(p, 4, menu.scroll[4], Val::Auto, |list| {
            setvar_rows(list, &items, false);
        });
        p.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|footer| {
            button(footer, "Reset", false, Action::ResetConfig);
            button(footer, "Done", true, Action::CloseConfig);
        });
    });
}

/// One row per option: the label with a stepper (choices and numbers) or a toggle (switches).
/// `live` makes the controls change the driven vehicle at once (the Setvars page) instead of the
/// choices of the vehicle that is about to be spawned.
/// The marker of an option that only takes effect when the vehicle is built again.
pub(in crate::in_game::vehicle_menu) fn respawn_marker() -> String {
    opts_gui::l10n::Localization::detect().t(opts_gui::l10n::wizard_keys::wiz::WIZ_SETVAR_RESPAWN)
}
