//! Hierarchy construction and view rendering for the map configuration modal.

use bevy::prelude::*;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use crate::map_profiles::advice::AdviceId;
use crate::map_profiles::profile::DEFAULT_CHRONO;
use crate::map_profiles::service::{Choice, ProfileView};
use crate::map_profiles::store::ProfileSource;

use super::{Action, MapConfigMenu, MapConfigRoot, NameField};

mod sections;
use sections::{
    advice_section, apply_buttons, choice_list, create_section, load_section, map_header,
    profile_section,
};

fn advice_key(id: AdviceId) -> &'static str {
    match id {
        AdviceId::GeometryAboveTypical => "map-config-advice-geometry-above-typical",
        AdviceId::GeometryHigh => "map-config-advice-geometry-high",
        AdviceId::LightMap => "map-config-advice-light-map",
        AdviceId::ManySplines => "map-config-advice-many-splines",
        AdviceId::HighAiLoad => "map-config-advice-high-ai-load",
        AdviceId::RanSlowly => "map-config-advice-ran-slowly",
        AdviceId::RanSmoothly => "map-config-advice-ran-smoothly",
        AdviceId::NotPlayedYet => "map-config-advice-not-played-yet",
    }
}

pub(super) fn arg(name: &str, value: impl ToString) -> (String, opts_gui::l10n::fluent::Arg) {
    (name.to_string(), value.to_string().into())
}

fn text(p: &mut ChildSpawnerCommands, s: impl Into<String>, size: f32, role: ColorRole) {
    p.spawn((
        Text::new(s.into()),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        ForegroundRole(role),
    ));
}

fn button(p: &mut ChildSpawnerCommands, label: &str, filled: bool, action: Action, id: String) {
    let b = if filled {
        ButtonBuilder::filled(label)
    } else {
        ButtonBuilder::tonal(label)
    };
    let e = b.spawn_child(p);
    p.commands().entity(e).insert((action, UiId::new(id)));
}

fn choice_label(loc: &Localization, view: &ProfileView, c: &Choice) -> String {
    match c {
        Choice::Global => loc.t("map-config-choice-global"),
        Choice::Preset(id) => {
            let name = view
                .presets
                .iter()
                .find(|p| p.based_on.as_deref() == Some(id))
                .map_or(id.clone(), |p| p.name.clone());
            loc.t_args("map-config-choice-preset", &[arg("name", name)])
        }
        Choice::Saved(name) => {
            let chrono = view
                .saved
                .iter()
                .any(|(c, p)| p.name == *name && c != DEFAULT_CHRONO);
            let key = if chrono {
                "map-config-choice-saved-chrono"
            } else {
                "map-config-choice-saved"
            };
            loc.t_args(key, &[arg("name", name)])
        }
    }
}

/// Builds the modal again when the view or the selection changed.
pub fn rebuild(
    mut menu: ResMut<MapConfigMenu>,
    roots: Query<Entity, With<MapConfigRoot>>,
    mut commands: Commands,
    loc: Res<Localization>,
) {
    if !menu.open {
        for r in &roots {
            commands.entity(r).despawn();
        }
        menu.built = 0;
        return;
    }
    if menu.built == menu.fingerprint && !roots.is_empty() {
        return;
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    menu.built = menu.fingerprint;
    let Some(view) = menu.view.clone() else {
        return;
    };
    let (root, content) = ModalBuilder::new()
        .with_size_percent(56.0, 88.0)
        .with_width_range(520.0, 820.0)
        .spawn(&mut commands);
    commands.entity(root).insert(MapConfigRoot);
    let status = menu.status.clone();
    let selected = menu.selected.clone();
    let name = menu.name.clone();
    commands.entity(content).with_children(|p| {
        map_header(p, &loc, &view, status);
        load_section(p, &loc, &view);
        advice_section(p, &loc, &view);
        profile_section(p, &loc, &view);
        choice_list(p, &loc, &view, selected.as_ref());
        create_section(p, &loc, &view, name);
        apply_buttons(p, &loc);
    });
}
