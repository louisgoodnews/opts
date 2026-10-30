//! The sections of the map configuration modal.

use super::*;

/// Title, map folder, Chrono entries and the status.
pub(super) fn map_header(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    status: String,
) {
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
}

/// The load profile of the map.
pub(super) fn load_section(p: &mut ChildSpawnerCommands, loc: &Localization, view: &ProfileView) {
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
}

/// What the profile suggests.
pub(super) fn advice_section(p: &mut ChildSpawnerCommands, loc: &Localization, view: &ProfileView) {
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
        let role = if a.severity == super::super::super::advice::Severity::Warning {
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
}

/// Which profile applies now and which one is picked.
pub(super) fn profile_section(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
) {
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
}

/// The profiles to choose from.
pub(super) fn choice_list(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    selected: Option<&Choice>,
) {
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
            let label = choice_label(loc, view, c);
            button(
                list,
                &label,
                selected == Some(c),
                Action::Select(c.clone()),
                format!("map-config.choice.{i}"),
            );
        }
    });
}

/// The name field and the *Create* buttons.
pub(super) fn create_section(
    p: &mut ChildSpawnerCommands,
    loc: &Localization,
    view: &ProfileView,
    name: String,
) {
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
}

/// *Apply*, *Apply and keep*, *Delete* and *Cancel*.
pub(super) fn apply_buttons(p: &mut ChildSpawnerCommands, loc: &Localization) {
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
}
