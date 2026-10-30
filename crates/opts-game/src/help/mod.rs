//! Help: controls, guides, report an issue and About (`docs/src/roadmap/phase-9/screens.md` §6).
//!
//! F1 opens it (D-UI-06): the page in the shell, an overlay during a drive. *Report an issue* is
//! reachable from here and from the pause menu; the control-protocol command `report` is
//! unchanged.

mod about;
mod controls;
mod overlay;
mod params;
pub mod scroll;
pub mod vehicle_controls;

use bevy::prelude::*;
use opts_bevy::reporting::{request_report, ReportModalData, ReportModalState};
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::state::Selected;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

pub use overlay::{open_overlay, HelpOverlay};
pub use params::{F1Gate, ReportRequest, VehicleProbe};

use crate::app_state::AppState;
use crate::pause::Pause;
use crate::shell::nav::{Navigate, PendingDeepLink};
use crate::shell::{keys, spawn_screen_root, ShellContent};

/// Root of the Help page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpRoot;

/// The right-hand pane of the Help page.
#[derive(Component, Debug, Clone, Copy)]
pub struct HelpPane;

/// The tabs of the Help page.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpTab {
    /// Key table.
    #[default]
    Controls,
    /// Guides (not available yet).
    Guides,
    /// Report an issue.
    Report,
    /// Version and licences.
    About,
}

impl HelpTab {
    const ALL: [Self; 4] = [Self::Controls, Self::Guides, Self::Report, Self::About];

    fn key(self) -> &'static str {
        match self {
            Self::Controls => keys::HELP_TAB_CONTROLS,
            Self::Guides => keys::HELP_TAB_GUIDES,
            Self::Report => keys::HELP_TAB_REPORT,
            Self::About => keys::HELP_TAB_ABOUT,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Controls => "controls",
            Self::Guides => "guides",
            Self::Report => "report",
            Self::About => "about",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.id() == text)
    }
}

/// The open tab.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HelpState {
    /// Which tab.
    pub tab: HelpTab,
}

/// The *Report an issue* button of the page.
#[derive(Component, Debug, Clone, Copy)]
pub struct ReportButton;

/// Help plugin.
pub struct HelpPlugin;

impl Plugin for HelpPlugin {
    fn build(&self, app: &mut App) {
        crate::pause::ensure_report_state(app);
        app.init_resource::<HelpState>()
            .init_resource::<Pause>()
            .add_systems(
                Update,
                (
                    controls::sync_action_chips,
                    vehicle_controls::fill_vehicle_controls,
                ),
            )
            .add_systems(OnEnter(AppState::Help), spawn_help)
            .add_systems(OnExit(AppState::Help), despawn_help)
            .add_systems(
                Update,
                (tab_clicks, sync_tabs, sync_pane, report_click).run_if(in_state(AppState::Help)),
            )
            .add_systems(
                Update,
                (f1_opens_help, overlay::overlay_actions, scroll::scroll_help)
                    .run_if(not(in_state(AppState::Loading))),
            );
    }
}

fn spawn_help(
    mut commands: Commands,
    shell: Res<ShellContent>,
    mut deep: ResMut<PendingDeepLink>,
    mut state: ResMut<HelpState>,
) {
    if let Some(tab) = deep.0.take().as_deref().and_then(HelpTab::parse) {
        state.tab = tab;
    }
    let root = spawn_screen_root(&mut commands, &shell, "Help");
    commands.entity(root).insert((
        HelpRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            padding: UiRect::all(Val::Px(24.0)),
            column_gap: Val::Px(24.0),
            ..default()
        },
    ));
    commands.entity(root).with_children(|page| {
        page.spawn((
            Node {
                width: Val::Px(220.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            FocusRegion(Landmark::Detail),
        ))
        .with_children(|tabs| {
            tabs.spawn((
                Text::new(keys::HELP_TITLE),
                L10nKey(keys::HELP_TITLE),
                TextColor::default(),
                TypeStyleToken(TypeScale::Headline),
                ForegroundRole(ColorRole::OnSurface),
                Node {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                },
            ));
            for tab in HelpTab::ALL {
                let button = ButtonBuilder::text(tab.key())
                    .with_key(tab.key())
                    .spawn_child(tabs);
                tabs.commands().entity(button).insert((
                    tab,
                    UiId::new(format!("help.tab.{}", tab.id())),
                    Node {
                        min_height: Val::Px(44.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ));
            }
        });
        page.spawn((
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            HelpPane,
            scroll::HelpScroll,
        ));
    });
}

fn despawn_help(mut commands: Commands, roots: Query<Entity, With<HelpRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn tab_clicks(
    mut activated: MessageReader<Activated>,
    tabs: Query<&HelpTab>,
    mut state: ResMut<HelpState>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(tab) = tabs.get(*entity) {
            state.tab = *tab;
        }
    }
}

fn sync_tabs(
    state: Res<HelpState>,
    mut commands: Commands,
    tabs: Query<(Entity, &HelpTab, Has<Selected>)>,
) {
    for (entity, tab, selected) in &tabs {
        let want = *tab == state.tab;
        if want == selected && !state.is_changed() {
            continue;
        }
        let mut e = commands.entity(entity);
        if want {
            e.try_insert((
                Selected,
                BackgroundRole(ColorRole::SecondaryContainer),
                StateLayerRole(ColorRole::OnSecondaryContainer),
            ));
        } else {
            e.try_remove::<(Selected, BackgroundRole)>();
            e.try_insert((
                BackgroundColor(Color::NONE),
                StateLayerRole(ColorRole::OnSurface),
            ));
        }
    }
}

fn sync_pane(
    mut commands: Commands,
    state: Res<HelpState>,
    panes: Query<(Entity, Option<&Children>), With<HelpPane>>,
    added: Query<(), Added<HelpPane>>,
) {
    if !state.is_changed() && added.is_empty() {
        return;
    }
    for (pane, children) in &panes {
        for child in children.into_iter().flat_map(|c| c.iter()) {
            commands.entity(child).despawn();
        }
        let tab = state.tab;
        commands.entity(pane).with_children(|p| match tab {
            HelpTab::Controls => controls::spawn_controls(p),
            HelpTab::Guides => {
                p.spawn((
                    Text::new(keys::HELP_GUIDES_BODY),
                    L10nKey(keys::HELP_GUIDES_BODY),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyLarge),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
                opts_gui::not_yet::spawn_label(p);
            }
            HelpTab::Report => {
                p.spawn((
                    Text::new(keys::HELP_REPORT_BODY),
                    L10nKey(keys::HELP_REPORT_BODY),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyLarge),
                    ForegroundRole(ColorRole::OnSurface),
                ));
                let button = ButtonBuilder::filled(keys::HELP_REPORT_ACTION)
                    .with_key(keys::HELP_REPORT_ACTION)
                    .spawn_child(p);
                p.commands().entity(button).insert((
                    ReportButton,
                    UiId::new("help.report"),
                    Node {
                        align_self: AlignSelf::FlexStart,
                        min_height: Val::Px(44.0),
                        padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                ));
            }
            HelpTab::About => about::spawn_about(p),
        });
    }
}

fn report_click(
    mut activated: MessageReader<Activated>,
    buttons: Query<(), With<ReportButton>>,
    time: Res<Time<Virtual>>,
    mut data: ResMut<ReportModalData>,
    state: Res<State<ReportModalState>>,
    mut next: ResMut<NextState<ReportModalState>>,
) {
    for Activated(entity) in activated.read() {
        if buttons.contains(*entity) {
            request_report(&time, &mut data, &state, &mut next);
        }
    }
}

/// F1 opens Help: the page in the shell, the overlay (over a paused scene) in a drive.
pub fn f1_opens_help(
    input: F1Gate,
    state: Res<State<AppState>>,
    mut pause: ResMut<Pause>,
    mut time: ResMut<Time<Virtual>>,
    mut navigate: MessageWriter<Navigate>,
    mut commands: Commands,
) {
    let F1Gate {
        keys_in,
        action_keys,
        gate,
        overlays,
    } = input;
    if !opts_bevy::action_keys_of(&action_keys).just_pressed(&keys_in, "help")
        || gate.closed()
        || !overlays.is_empty()
    {
        return;
    }
    match state.get() {
        AppState::InGame => {
            if !pause.open {
                pause.resume_time = !time.is_paused();
                time.pause();
                pause.open = true;
            } else if let Some(root) = pause.root.take() {
                commands.entity(root).try_despawn();
            }
            open_overlay(&mut commands);
        }
        AppState::Help => {}
        _ => {
            navigate.write(Navigate::push("help"));
        }
    }
}

#[cfg(test)]
mod tests;
