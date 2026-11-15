//! The markers and the state components of the HUD: the roots, slots, rows, readouts and texts.

use super::*;

/// Marks the root of the HUD.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HudRoot;

/// Marks the panel of a slot.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudSlot(pub Slot);

/// Marks the panel of a row of the bottom-left cluster.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudRow(pub Row);

/// The bottom-left cluster and its rows: the layout caps their width so that the cluster never
/// reaches the mini map.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ClusterBox;

/// Marks the mini map's panel: the layout sets its width.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MapBox;

/// Marks the row that holds the camera label: the layout puts it below the status bar, however
/// tall that is (larger text makes it taller).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CameraLabelRow;

/// A readout column whose minimum width depends on the layout (fixed width, so values do not
/// jitter; narrower in a compact layout).
#[derive(Component, Debug, Clone, Copy)]
pub struct ReadoutBox {
    /// Width in a normal layout (layout px).
    pub normal: f32,
    /// Width in a compact layout.
    pub compact: f32,
}

impl ReadoutBox {
    /// The width for the layout.
    #[must_use]
    pub const fn width(&self, compact: bool) -> f32 {
        if compact {
            self.compact
        } else {
            self.normal
        }
    }
}

/// Marks the root node of a widget.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudWidgetNode {
    /// The widget's id.
    pub id: &'static str,
    /// Where it was built.
    pub slot: Slot,
}

/// Which part of a widget a text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The value.
    Value,
    /// The caption or unit.
    Caption,
}

/// A text the update systems write.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudText {
    /// What it shows.
    pub binding: Binding,
    /// Value or caption.
    pub part: Part,
}

/// The pill of the alert slot.
#[derive(Component, Debug, Clone, Copy)]
pub struct AlertPill;

/// The profile the running HUD was built from.
#[derive(Resource, Debug, Clone)]
pub struct ActiveProfile {
    /// The kind.
    pub kind: VehicleKind,
    /// The profile.
    pub profile: HudProfile,
}
