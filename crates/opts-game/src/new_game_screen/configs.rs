//! The *Configurations* section of the *What* step (M-0077): the saved bus configurations as a
//! list with *Load*, favourite, *Duplicate* and *Delete* (asks once more), and a name field with
//! *Save* for the vehicle, repaint and options of the plan.
//!
//! The list is kept in memory ([`ConfigLibrary`], read at startup); every change is written at
//! once, a few hundred bytes on a click. The control protocol (`control_configs.rs`) works on the
//! same resource, so the screen and a script see one list.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::interaction::events::TextChanged;
use opts_gui::l10n::{L10nArgs, Localization};
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::PlannerStep;
use super::repaints::VehicleRepaints;
use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, keyed_args, plain, row, Emphasis, PlannerAction};
use crate::bus_config::{BusConfiguration, BusConfigurations, Problem, StoreProblem};

mod parts;
use parts::{name_row, problem_lines, row_buttons};

/// The saved configurations as the wizard keeps them during a session.
#[derive(Resource, Debug, Clone)]
pub struct ConfigLibrary {
    /// The configurations (file `bus_configurations.json` beside the settings database).
    pub store: BusConfigurations,
    /// Where they are saved; `None` keeps them in memory only (tests, no config directory).
    pub path: Option<PathBuf>,
    /// Why the file could not be used (damaged, newer version); a newer file is never written.
    pub problem: Option<StoreProblem>,
    /// The text of the name field.
    pub draft_name: String,
    /// The configuration whose *Delete* was pressed once.
    pub pending_delete: Option<String>,
    /// The configuration just loaded, whose repaint and options still wait for the vehicle's
    /// repaint data to be checked against (see [`finish_load`]).
    pub pending_check: Option<String>,
}

impl Default for ConfigLibrary {
    fn default() -> Self {
        Self::at(BusConfigurations::path())
    }
}

impl ConfigLibrary {
    /// A library stored at `path` (or only in memory).
    #[must_use]
    pub fn at(path: Option<PathBuf>) -> Self {
        let (store, problem) = match path.as_deref().map(BusConfigurations::load_from) {
            Some(Ok(s)) => (s, None),
            Some(Err(p)) => (BusConfigurations::default(), Some(p)),
            None => (BusConfigurations::default(), None),
        };
        Self {
            store,
            path,
            problem,
            draft_name: String::new(),
            pending_delete: None,
            pending_check: None,
        }
    }

    /// Writes the list. Nothing is written over a file of a newer version.
    ///
    /// # Errors
    /// The I/O error of the write, or a refusal to overwrite a newer file.
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(StoreProblem::Newer(v)) = &self.problem {
            return Err(std::io::Error::other(format!(
                "the configurations file has version {v}; it is not overwritten"
            )));
        }
        let Some(p) = &self.path else {
            return Ok(());
        };
        self.store.save_to(p)?;
        // A damaged file was moved aside by the write: the list is a good one now.
        self.problem = None;
        Ok(())
    }
}

/// The name field.
#[derive(Component)]
pub struct ConfigNameField;

/// Keeps the name field's text in the library.
pub fn track_name(
    mut edited: MessageReader<TextChanged>,
    fields: Query<&ConfigNameField>,
    mut library: ResMut<ConfigLibrary>,
) {
    for t in edited.read() {
        if fields.get(t.entity).is_ok() {
            library.draft_name.clone_from(&t.text);
        }
    }
}

/// The name a configuration gets when the field is empty: the vehicle and the repaint.
#[must_use]
pub fn default_name(vehicle: &VehicleItem, repaint: &str) -> String {
    if repaint.is_empty() {
        vehicle.full_name.clone()
    } else {
        format!("{} – {repaint}", vehicle.full_name)
    }
}

/// What the screen shows of a configuration's check, as a message key and its variable.
fn problem_text(p: &Problem) -> (&'static str, String) {
    match p {
        Problem::VehicleMissing { .. } => ("wiz-configs-vehicle-missing", String::new()),
        Problem::RepaintMissing { repaint } => ("wiz-configs-repaint-missing", repaint.clone()),
        Problem::SetvarUnknown { name } | Problem::SetvarRejected { name, .. } => {
            ("wiz-configs-option-left-out", name.clone())
        }
    }
}

/// Builds the section: title, name field with *Save*, then the list.
pub fn spawn_configs_section(
    commands: &mut Commands,
    loc: &Localization,
    library: &ConfigLibrary,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
    repaints: Option<&VehicleRepaints>,
) -> Entity {
    let section = commands.spawn(column(8.0)).id();
    let mut kids = vec![commands
        .spawn(keyed(
            "wiz-configs-title",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id()];
    if let Some(problem) = &library.problem {
        kids.push(
            commands
                .spawn(keyed_args(
                    "wiz-configs-file-problem",
                    L10nArgs::one("reason", problem.to_string()),
                    TypeScale::BodyMedium,
                    ColorRole::Warning,
                ))
                .id(),
        );
    }
    let name_row = name_row(commands, loc, library, setup, vehicle);
    kids.push(name_row);

    if library.store.configurations.is_empty() {
        kids.push(
            commands
                .spawn(keyed(
                    "wiz-configs-empty",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    // Favourites first, then the most recently used.
    let mut order: Vec<&BusConfiguration> = library.store.configurations.iter().collect();
    order.sort_by_key(|c| (!c.favourite, std::cmp::Reverse(c.last_used)));
    for (i, c) in order.into_iter().enumerate() {
        kids.push(config_row(
            commands,
            library,
            catalogue,
            c,
            (vehicle, repaints),
            i,
        ));
    }
    commands.entity(section).add_children(&kids);
    section
}

fn config_row(
    commands: &mut Commands,
    library: &ConfigLibrary,
    catalogue: &GameCatalogue,
    c: &BusConfiguration,
    shown: (&VehicleItem, Option<&VehicleRepaints>),
    index: usize,
) -> Entity {
    let line = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(8.0)
        })
        .id();
    let vehicle = catalogue.vehicles.iter().find(|v| v.id == c.vehicle);
    // The check needs the repaints of the configuration's vehicle; only the shown vehicle's are
    // at hand. For another vehicle, or while they are being read, only a missing vehicle is
    // shown here (the full check runs when the configuration is loaded).
    let known = shown.1.filter(|_| shown.0.id == c.vehicle);
    let mut checked = c.check(catalogue, |_| known.cloned().unwrap_or_default());
    if known.is_none() {
        checked.problems.retain(Problem::is_fatal);
    }
    let load = spawn_choice(
        commands,
        PlannerAction::LoadConfig(c.id.clone()),
        format!("wizard.what.configs.{index}.load"),
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
            ..default()
        },
    );
    let mut title = c.name.clone();
    if c.favourite {
        title.insert_str(0, "★ ");
    }
    let sub = match vehicle {
        Some(v) if c.repaint.is_empty() => v.full_name.clone(),
        Some(v) => format!("{} · {}", v.full_name, c.repaint),
        None => c.vehicle.clone(),
    };
    let name = commands
        .spawn(plain(title, TypeScale::BodyLarge, ColorRole::OnSurface))
        .id();
    let subtitle = commands
        .spawn(plain(
            sub,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let mut content = vec![name, subtitle];
    content.extend(problem_lines(commands, &checked.problems));
    commands
        .entity(load)
        .insert(ChoiceStep(PlannerStep::What))
        .insert_children(1, &content);
    let [favourite, duplicate, delete] = row_buttons(commands, library, c, index);
    commands
        .entity(line)
        .add_children(&[load, favourite, duplicate, delete]);
    line
}

/// After *Load*: when the vehicle's repaints are known, checks the loaded repaint and options
/// against them, leaves out what the vehicle no longer offers and says so.
pub fn finish_load(
    mut library: ResMut<ConfigLibrary>,
    liveries: Res<super::livery::LiveryState>,
    catalogue: Res<GameCatalogue>,
    mut setup: ResMut<NewGameSetup>,
    loc: Res<Localization>,
    mut commands: Commands,
) {
    let Some(id) = library.pending_check.clone() else {
        return;
    };
    let Some(config) = library.store.find(&id).cloned() else {
        library.pending_check = None;
        return;
    };
    let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == config.vehicle) else {
        library.pending_check = None;
        return;
    };
    if setup.selected_vehicle != catalogue.vehicle_key(vehicle) {
        // The user chose another vehicle meanwhile.
        library.pending_check = None;
        return;
    }
    let Some(reps) = liveries
        .repaints
        .as_ref()
        .filter(|_| liveries.vehicle == vehicle.id)
    else {
        return;
    };
    library.pending_check = None;
    let checked = config.check(&catalogue, |_| reps.clone());
    setup.selected_livery.clone_from(&checked.repaint);
    setup.custom_setvars = checked.setvars.iter().cloned().collect();
    let key = if checked.problems.is_empty() {
        "wiz-configs-loaded"
    } else {
        "wiz-configs-loaded-partly"
    };
    let args = [
        ("name".to_string(), config.name.clone().into()),
        ("count".to_string(), checked.problems.len().into()),
    ];
    ToastBuilder::new(loc.t_args(key, &args))
        .with_duration(5.0)
        .spawn(&mut commands);
}
