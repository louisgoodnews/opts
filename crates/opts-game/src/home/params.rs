//! System-parameter bundles of the Home systems.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_gui::l10n::{L10nKey, Localization};

use super::{HomeButton, StatusChip};
use crate::catalogue_service::CatalogueService;
use crate::new_game_screen::types::GameCatalogue;

/// What Home knows about the installation.
#[derive(SystemParam)]
pub struct HomeFacts<'w> {
    /// The catalogue service, once started.
    pub service: Option<Res<'w, CatalogueService>>,
    /// The maps and vehicles read so far.
    pub lists: Res<'w, GameCatalogue>,
    /// The background look at the installation.
    pub facts: Res<'w, crate::content_facts::ContentFactsStore>,
    /// The texts.
    pub loc: Res<'w, Localization>,
}

/// The widgets of Home that follow the installation status.
#[derive(SystemParam)]
pub struct HomeWidgets<'w, 's> {
    /// The status chips.
    pub chips: Query<'w, 's, (Entity, &'static mut L10nKey), With<StatusChip>>,
    /// The drive buttons.
    pub buttons: Query<'w, 's, (Entity, &'static HomeButton)>,
    /// A chip was just added (the page was rebuilt).
    pub fresh: Query<'w, 's, (), Added<StatusChip>>,
}
