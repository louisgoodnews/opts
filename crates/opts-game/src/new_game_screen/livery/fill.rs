//! Fills the livery and option boxes of the vehicle detail from the discovered liveries.

use super::*;

/// Empties `container` and puts the livery buttons in it.
pub fn fill_liveries(
    commands: &mut Commands,
    container: Entity,
    vehicle: &VehicleItem,
    reps: &VehicleRepaints,
) {
    commands.entity(container).despawn_related::<Children>();
    if reps.names.len() < 2 {
        let line = commands
            .spawn(keyed(
                "wiz-what-livery-none",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(container).add_child(line);
        return;
    }
    let mut kids = Vec::new();
    for (i, name) in reps.names.iter().enumerate() {
        let button = spawn_choice(
            commands,
            PlannerAction::Livery(name.clone()),
            format!("wizard.what.livery.{i}"),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                ..default()
            },
        );
        let swatch = commands
            .spawn((
                Node {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundRole(ColorRole::SurfaceContainerHighest),
                BorderRole(ColorRole::Outline),
                RadiusToken(RadiusLevel::Full),
                LiverySwatch {
                    vehicle: vehicle.id.clone(),
                    livery: name.clone(),
                },
            ))
            .id();
        let label = commands
            .spawn(plain(
                name.clone(),
                TypeScale::BodyMedium,
                ColorRole::OnSurface,
            ))
            .id();
        let mut content = vec![swatch, label];
        if *name == vehicle.default_livery {
            content.push(
                commands
                    .spawn(keyed(
                        "wiz-what-livery-default",
                        TypeScale::Label,
                        ColorRole::OnSurfaceVariant,
                    ))
                    .id(),
            );
        }
        commands
            .entity(button)
            .insert(ChoiceStep(PlannerStep::What))
            .insert_children(1, &content);
        kids.push(button);
    }
    commands.entity(container).add_children(&kids);
}

/// Empties `container` and puts *Configure…* or the no-options line in it.
pub fn fill_options(
    commands: &mut Commands,
    container: Entity,
    reps: &VehicleRepaints,
    setup: &NewGameSetup,
) {
    commands.entity(container).despawn_related::<Children>();
    let items = setvars_from(reps, &setup.custom_setvars, &setup.selected_livery);
    if items.is_empty() {
        let line = commands
            .spawn(keyed(
                "wiz-what-no-options",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(container).add_child(line);
        return;
    }
    let changed = setup.custom_setvars.len();
    let button = action_button(
        commands,
        "wiz-what-configure",
        Emphasis::Tonal,
        PlannerAction::OpenOptions,
        "wizard.what.configure".to_string(),
    );
    let mut kids = vec![button];
    if changed > 0 {
        kids.push(
            commands
                .spawn(keyed_args(
                    "wiz-what-options-changed",
                    opts_gui::l10n::L10nArgs::one("count", changed),
                    TypeScale::Label,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    commands.entity(container).add_children(&kids);
}
