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
        text(p, loc.t("map-config-title"), 22.0, ColorRole::Primary);
        text(p, view.map_dir_name.clone(), 15.0, ColorRole::OnSurface);
        let chrono = if view.chrono_entries.is_empty() {
            loc.t("map-config-chrono-none")
        } else {
            let newest = view.chrono_entries.last().cloned().unwrap_or_default();
            let others = view.chrono_entries.len() - 1;
            if others == 0 {
                loc.t_args("map-config-chrono-one", &[arg("newest", newest)])
            } else {
                loc.t_args(
                    "map-config-chrono",
                    &[arg("newest", newest), arg("others", others)],
                )
            }
        };
        text(p, chrono, 13.0, ColorRole::OnSurfaceVariant);
        if !status.is_empty() {
            text(p, status, 13.0, ColorRole::Primary);
        }

        // Load profile and advice.
        text(p, loc.t("map-config-load"), 16.0, ColorRole::Primary);
        if view.load.tile_count == 0 {
            text(
                p,
                loc.t("map-config-counting"),
                13.0,
                ColorRole::OnSurfaceVariant,
            );
        } else {
            let l = &view.load;
            let mut line = loc.t_args(
                "map-config-load-counts",
                &[
                    arg("objects", l.object_count),
                    arg("splines", l.spline_count),
                    arg("tiles", l.tile_count),
                    arg("area", format!("{:.1}", l.map_size_km2)),
                    arg("pool", l.ai_pool_vehicles),
                ],
            );
            if view.counting {
                line.push(' ');
                line.push_str(&loc.t("map-config-load-estimate"));
            }
            text(p, line, 13.0, ColorRole::OnSurface);
            let last = match (l.avg_frame_time_ms, l.p95_frame_time_ms) {
                (Some(a), Some(b)) => loc.t_args(
                    "map-config-last",
                    &[arg("avg", format!("{a:.0}")), arg("p95", format!("{b:.0}"))],
                ),
                _ => loc.t("map-config-never"),
            };
            text(p, last, 13.0, ColorRole::OnSurfaceVariant);
        }
        text(p, loc.t("map-config-advice"), 16.0, ColorRole::Primary);
        if view.advice.is_empty() {
            text(
                p,
                loc.t("map-config-advice-none"),
                13.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for a in &view.advice {
            let role = if a.severity == super::super::advice::Severity::Warning {
                ColorRole::Warning
            } else {
                ColorRole::OnSurface
            };
            text(p, loc.t(advice_key(a.id)), 13.0, role);
            if let Some(preset) = a.preset {
                let name = view
                    .presets
                    .iter()
                    .find(|x| x.based_on.as_deref() == Some(preset))
                    .map_or(preset.to_string(), |x| x.name.clone());
                text(
                    p,
                    loc.t_args("map-config-preset-hint", &[arg("preset", name)]),
                    12.0,
                    ColorRole::OnSurfaceVariant,
                );
            }
        }

        // Profiles.
        text(p, loc.t("map-config-profile"), 16.0, ColorRole::Primary);
        let resolved = match (&view.resolved.profile, view.resolved.source) {
            (Some(pr), ProfileSource::Chrono) => {
                loc.t_args("map-config-resolved-chrono", &[arg("name", &pr.name)])
            }
            (Some(pr), _) => loc.t_args("map-config-resolved-map", &[arg("name", &pr.name)]),
            (None, _) => loc.t("map-config-resolved-global"),
        };
        text(p, resolved, 13.0, ColorRole::OnSurfaceVariant);
        match &view.picked {
            Some(Some(n)) => text(
                p,
                loc.t_args("map-config-picked", &[arg("name", n)]),
                13.0,
                ColorRole::Primary,
            ),
            Some(None) => text(
                p,
                loc.t("map-config-picked-global"),
                13.0,
                ColorRole::Primary,
            ),
            None => {}
        }
        let mut choices = vec![Choice::Global];
        choices.extend(
            view.presets
                .iter()
                .filter_map(|x| x.based_on.clone().map(Choice::Preset)),
        );
        choices.extend(
            view.saved
                .iter()
                .map(|(_, x)| Choice::Saved(x.name.clone())),
        );
        p.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            overflow: Overflow::scroll_y(),
            ..default()
        })
        .with_children(|list| {
            for (i, c) in choices.iter().enumerate() {
                let label = choice_label(&loc, &view, c);
                button(
                    list,
                    &label,
                    selected.as_ref() == Some(c),
                    Action::Select(c.clone()),
                    format!("map-config.choice.{i}"),
                );
            }
        });

        // Create.
        text(
            p,
            loc.t("map-config-name"),
            13.0,
            ColorRole::OnSurfaceVariant,
        );
        let field = TextFieldBuilder::outlined(loc.t("map-config-name"))
            .with_text(name)
            .with_max_characters(60)
            .spawn_child(p);
        p.commands()
            .entity(field)
            .insert((NameField, UiId::new("map-config.name")));
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            button(
                row,
                &loc.t("map-config-create"),
                false,
                Action::Create { chrono: false },
                "map-config.create".to_string(),
            );
            if view.key.chrono_id != DEFAULT_CHRONO {
                button(
                    row,
                    &loc.t("map-config-create-chrono"),
                    false,
                    Action::Create { chrono: true },
                    "map-config.create-chrono".to_string(),
                );
            }
        });
        p.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|row| {
            button(
                row,
                &loc.t("map-config-apply"),
                true,
                Action::Apply { keep: false },
                "map-config.apply".to_string(),
            );
            button(
                row,
                &loc.t("map-config-apply-keep"),
                false,
                Action::Apply { keep: true },
                "map-config.apply-keep".to_string(),
            );
            button(
                row,
                &loc.t("map-config-delete"),
                false,
                Action::Delete,
                "map-config.delete".to_string(),
            );
            button(
                row,
                &loc.t("map-config-cancel"),
                false,
                Action::Close,
                "map-config.cancel".to_string(),
            );
        });
    });
}
