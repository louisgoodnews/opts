//! What the scan status chip is worked out from, bundled for its refresh system.

use bevy::ecs::system::SystemParam;

use super::*;

/// The texts, the settings, the catalogue service and the background facts.
#[derive(SystemParam)]
pub struct ChipFacts<'w> {
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The settings (the last scan's time).
    pub settings: Option<Res<'w, GameSettings>>,
    /// The catalogue service, once started.
    pub service: Option<Res<'w, CatalogueService>>,
    /// The background look at the installation.
    pub facts: Res<'w, ContentFactsStore>,
}
