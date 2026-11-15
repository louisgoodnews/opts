//! The queries and the resources the door camera overlay system reads.

use super::*;

pub(super) type Roots<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut DoorOverlay,
        &'static mut Node,
        &'static mut InstrumentFade,
    ),
>;

/// The resources [`update_overlay`] reads.
#[derive(SystemParam)]
pub struct OverlayEnv<'w> {
    /// Real time.
    pub time: Res<'w, Time<Real>>,
    /// The door cameras' slots.
    pub cam: Res<'w, DoorCamera>,
    /// The HUD layout.
    pub layout: Res<'w, HudLayout>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The theme, once there.
    pub theme: Option<Res<'w, MaterialTheme>>,
    /// The instrument variant.
    pub variant: Res<'w, InstrumentVariant>,
    /// The viewpoint editor's state.
    pub editor: Option<Res<'w, opts_bevy::ViewpointState>>,
}
