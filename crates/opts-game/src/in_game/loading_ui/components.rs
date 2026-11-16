//! Marker components of the loading screen and the three manifest tiles.

use bevy::prelude::*;

/// *Details* button of the outcome block.
#[derive(Component, Debug, Clone, Copy)]
pub struct DetailsBtn;
/// *Continue* button of *Ready with problems*.
#[derive(Component, Debug, Clone, Copy)]
pub struct ContinueBtn;
/// *Retry* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct RetryBtn;
/// *Choose another* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct ChooseAnotherBtn;
/// *Back* button of *Failed*.
#[derive(Component, Debug, Clone, Copy)]
pub struct LoadingBackBtn;
/// The stage sentence.
#[derive(Component, Debug, Clone, Copy)]
pub struct StageText;
/// The percentage text.
#[derive(Component, Debug, Clone, Copy)]
pub struct PercentText;
/// The container the outcome (result line, problems, buttons) is built into.
#[derive(Component, Debug, Clone, Copy)]
pub struct OutcomeBox;
/// A text of the manifest, by tile and line.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestText(pub Tile, pub u8);
/// The *Cancel loading?* dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct CancelDialog;
/// *Keep loading* in the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct KeepLoadingBtn;
/// *Cancel loading* in the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct ConfirmCancelBtn;

/// The three tiles of the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    /// Map and start point.
    Map,
    /// Vehicle and livery.
    Vehicle,
    /// Date, time and weather.
    When,
}
