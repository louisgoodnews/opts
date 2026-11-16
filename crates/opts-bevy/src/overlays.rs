//! Weather overlays of vehicle bodies and windows: the setting "Window and body overlays"
//! (`graphics.overlays`, M-0120).
//!
//! The overlays are the alpha parts of a model whose `[alphascale]` variable is a weather class
//! ([`overlay_class`]): rain on the windows and condensation (window overlays), dirt and the
//! precipitation mirror (body overlays).
//!
//! * [`OverlayLevel::Off`]: no overlay part is built and any built one is despawned.
//! * [`OverlayLevel::Simple`]: window overlays are kept, body overlays are not built.
//! * [`OverlayLevel::Full`]: both are kept (default).
//!
//! The level is process-wide ([`set_overlay_level`]) because the vehicle loader runs on tasks
//! without access to the ECS world. A raised level applies to vehicles loaded afterwards; a
//! lowered level removes the overlays of vehicles already on the road ([`despawn_disallowed`]).

use std::sync::atomic::{AtomicU8, Ordering};

use bevy::prelude::*;

use crate::model::{overlay_class, ModelPart, OverlayClass};

/// How many weather overlays are built (the setting "Window and body overlays").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayLevel {
    /// No overlay entities at all.
    Off,
    /// Window overlays only.
    Simple,
    /// Window and body overlays (default).
    #[default]
    Full,
}

/// Which part of the vehicle an overlay lies on.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    /// Rain on the windows (`Rain_Window_*`) and condensation (`Beschlag_*`).
    Window,
    /// Dirt (`Dirt_*`) and the precipitation mirror (`Precip_*`).
    Body,
}

impl OverlayLevel {
    /// The stored text of each level, in menu order.
    pub const ALL: [Self; 3] = [Self::Off, Self::Simple, Self::Full];

    /// The stored text (`off`, `simple`, `full`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Simple => "simple",
            Self::Full => "full",
        }
    }

    /// Parses a stored text; `None` for an unknown one.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|l| l.name().eq_ignore_ascii_case(text.trim()))
    }

    /// Whether an overlay of `kind` is built at this level.
    #[must_use]
    pub fn keeps(self, kind: OverlayKind) -> bool {
        match (self, kind) {
            (Self::Off, _) => false,
            (Self::Simple, OverlayKind::Window) | (Self::Full, _) => true,
            (Self::Simple, OverlayKind::Body) => false,
        }
    }

    fn to_byte(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Simple => 1,
            Self::Full => 2,
        }
    }

    fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Off,
            1 => Self::Simple,
            _ => Self::Full,
        }
    }
}

impl OverlayKind {
    /// The kind of the overlay that follows `variable` (`None` for a variable that is no
    /// weather class).
    #[must_use]
    pub fn of_variable(variable: &str) -> Option<Self> {
        match overlay_class(variable)? {
            OverlayClass::RainWindow | OverlayClass::Condensation => Some(Self::Window),
            OverlayClass::Dirt | OverlayClass::Precipitation => Some(Self::Body),
        }
    }
}

static LEVEL: AtomicU8 = AtomicU8::new(2);

/// The process-wide overlay level (`Full` until the settings say otherwise).
#[must_use]
pub fn overlay_level() -> OverlayLevel {
    OverlayLevel::from_byte(LEVEL.load(Ordering::Relaxed))
}

/// Sets the process-wide overlay level (the live "Window and body overlays" setting).
pub fn set_overlay_level(level: OverlayLevel) {
    LEVEL.store(level.to_byte(), Ordering::Relaxed);
}

/// Whether the loader builds `part` at the process-wide level: every part except a weather
/// overlay the level drops.
#[must_use]
pub fn part_is_built(part: &ModelPart) -> bool {
    part_kept(overlay_level(), part)
}

/// Whether `level` keeps `part` (see [`part_is_built`]).
#[must_use]
pub fn part_kept(level: OverlayLevel, part: &ModelPart) -> bool {
    part_kind(part).is_none_or(|kind| level.keeps(kind))
}

/// The overlay kind of a loaded part (`None` for every part that is no weather overlay).
#[must_use]
pub fn part_kind(part: &ModelPart) -> Option<OverlayKind> {
    part.alpha_variable
        .as_deref()
        .and_then(OverlayKind::of_variable)
}

/// Despawns the overlay entities the level `level` does not keep; returns how many.
pub fn despawn_disallowed(
    commands: &mut Commands,
    level: OverlayLevel,
    overlays: &Query<(Entity, &OverlayKind)>,
) -> usize {
    let mut removed = 0;
    for (entity, kind) in overlays {
        if !level.keeps(*kind) {
            commands.entity(entity).despawn();
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests;
