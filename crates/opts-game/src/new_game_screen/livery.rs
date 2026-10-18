//! Liveries and equipment options of the chosen vehicle, discovered off the main thread.

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::PlannerStep;
use super::repaints::{vehicle_repaints, VehicleRepaints};
use super::setvars_discovery::setvars_from;
use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, keyed_args, plain, row, Emphasis, PlannerAction};

/// The liveries and options found for one vehicle.
#[derive(Resource, Default)]
pub struct LiveryState {
    /// Vehicle (id) the data belongs to.
    pub vehicle: String,
    /// The repaints, once discovered.
    pub repaints: Option<VehicleRepaints>,
    task: Option<(String, Task<VehicleRepaints>)>,
}

/// Container of the livery buttons in the vehicle detail.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LiveryBox(pub String);

/// Container of the *Configure…* button (or the no-options line).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct OptionsBox(pub String);

/// The swatch of a livery button; the thumbnail service colours it from the livery's texture.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct LiverySwatch {
    /// Vehicle id.
    pub vehicle: String,
    /// Livery name.
    pub livery: String,
}

/// Starts discovery for `vehicle` unless it is cached or running.
pub fn request(state: &mut LiveryState, vehicle: &VehicleItem) {
    if state.vehicle == vehicle.id && (state.repaints.is_some() || state.task.is_some()) {
        return;
    }
    state.vehicle.clone_from(&vehicle.id);
    state.repaints = None;
    let v = vehicle.clone();
    state.task = Some((
        vehicle.id.clone(),
        IoTaskPool::get().spawn(async move { vehicle_repaints(&v) }),
    ));
}

/// Collects finished discovery and fills the boxes of the vehicle.
pub fn poll_liveries(
    mut state: ResMut<LiveryState>,
    catalogue: Res<GameCatalogue>,
    setup: Res<NewGameSetup>,
    boxes: Query<(Entity, &LiveryBox)>,
    options: Query<(Entity, &OptionsBox)>,
    mut commands: Commands,
) {
    let finished = match state.task.as_mut().and_then(|(_, t)| check_ready(t)) {
        Some(r) => r,
        None => return,
    };
    let Some((id, _)) = state.task.take() else {
        return;
    };
    if id != state.vehicle {
        return;
    }
    state.repaints = Some(finished);
    let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == id) else {
        return;
    };
    let reps = state.repaints.clone().unwrap_or_default();
    for (e, b) in &boxes {
        if b.0 == id {
            fill_liveries(&mut commands, e, vehicle, &reps);
        }
    }
    for (e, b) in &options {
        if b.0 == id {
            fill_options(&mut commands, e, &reps, &setup);
        }
    }
}

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

/// The livery section of the detail: title, loading line and the boxes the poll system fills.
pub fn spawn_livery_section(
    commands: &mut Commands,
    state: &mut LiveryState,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
) -> Entity {
    request(state, vehicle);
    let section = commands.spawn(column(8.0)).id();
    let title = commands
        .spawn(keyed(
            "wiz-what-livery",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let liveries = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Stretch,
                ..row(8.0)
            },
            LiveryBox(vehicle.id.clone()),
        ))
        .id();
    let options = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                ..row(12.0)
            },
            OptionsBox(vehicle.id.clone()),
        ))
        .id();
    match (&state.repaints, state.vehicle == vehicle.id) {
        (Some(reps), true) => {
            let reps = reps.clone();
            fill_liveries(commands, liveries, vehicle, &reps);
            fill_options(commands, options, &reps, setup);
        }
        _ => {
            let loading = commands
                .spawn(keyed(
                    "wiz-what-livery-loading",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id();
            commands.entity(liveries).add_child(loading);
        }
    }
    commands
        .entity(section)
        .add_children(&[title, liveries, options]);
    section
}
