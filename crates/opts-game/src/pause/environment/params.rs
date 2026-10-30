//! System-parameter bundles of the *Environment* page's systems.

use bevy::ecs::system::SystemParam;

use super::*;

/// The messages the page reacts to.
#[derive(SystemParam)]
pub struct PageEvents<'w, 's> {
    /// Button activations.
    pub activated: MessageReader<'w, 's, Activated>,
    /// Modal events (Esc).
    pub modal_events: MessageReader<'w, 's, ModalEvent>,
}

/// The nodes the page's input looks up.
#[derive(SystemParam)]
pub struct PageNodes<'w, 's> {
    /// The actions of the controls.
    pub actions: Query<'w, 's, (&'static EnvAction, Option<&'static WeatherId>)>,
    /// The page's overlay.
    pub roots: Query<'w, 's, Entity, With<EnvironmentOverlay>>,
    /// The modal roots.
    pub modal_roots: Query<'w, 's, (), With<ModalRoot>>,
}

/// What the page's values are worked out from.
#[derive(SystemParam)]
pub struct PageFacts<'w> {
    /// The page's state.
    pub page: Res<'w, EnvironmentPage>,
    /// The session's environment.
    pub env: Res<'w, SessionEnvironment>,
    /// The map rebuild for a new date.
    pub rebuild: Res<'w, ChronoRebuild>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The weather catalogue.
    pub weather: Res<'w, WeatherCatalogue>,
}
