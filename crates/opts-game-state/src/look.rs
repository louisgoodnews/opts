//! The colour *look* of the 3D view, as types: [`Look`] (the user setting) and [`LookParams`]
//! (what a look does to the camera and the sky). The systems that apply them stay in `opts-game`.
//!
//! Provisional values, see M-0060 step 1 (physical calibration follows in step 2): *Vivid* is the
//! default, *Bevy default* reproduces the behaviour before the looks existed.
//!
//! Spec: `docs/src/architecture/texture-quality.md` §3.2 and `docs/src/roadmap/phase-8/M-0060.md`.

use bevy::render::view::Tonemapping;
use serde::{Deserialize, Serialize};

/// A named colour look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Look {
    /// Richer saturation and contrast than reality-neutral (default).
    #[default]
    Vivid,
    /// Bevy's default tonemapper with the current ambient light; the base for the physical
    /// calibration of M-0060 step 2.
    Natural,
    /// Exactly the behaviour before looks existed.
    BevyDefault,
}

impl Look {
    /// All looks in menu order.
    pub const ALL: [Self; 3] = [Self::Vivid, Self::Natural, Self::BevyDefault];

    /// The stable key used in settings, the control protocol and `--look`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Vivid => "vivid",
            Self::Natural => "natural",
            Self::BevyDefault => "bevy",
        }
    }

    /// The label shown in the settings screen.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vivid => "Vivid (default)",
            Self::Natural => "Natural",
            Self::BevyDefault => "Bevy default",
        }
    }

    /// Parses a [`key`](Self::key) (also `bevy_default`, `bevy-default`; case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "vivid" => Some(Self::Vivid),
            "natural" => Some(Self::Natural),
            "bevy" | "bevy_default" | "bevy-default" | "bevydefault" => Some(Self::BevyDefault),
            _ => None,
        }
    }
}

/// What a [`Look`] does to the camera and the sky.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookParams {
    /// Tonemapper of the main camera.
    pub tonemapping: Tonemapping,
    /// Saturation applied after tonemapping (`1.0` = unchanged).
    pub post_saturation: f32,
    /// Contrast of shadows, midtones and highlights (`1.0` = unchanged).
    pub contrast: f32,
    /// Factor on the ambient light the sky system computes (`1.0` = unchanged).
    pub ambient_scale: f32,
}

impl Look {
    /// The parameters of this look (provisional values, M-0060 step 1).
    ///
    /// *Natural* and *Bevy default* are identical until the physical calibration of step 2 gives
    /// *Natural* its own sun, sky and exposure.
    #[must_use]
    pub fn params(self) -> LookParams {
        match self {
            // Measured on the reference view (texture-quality.md 3.2): +40 % mean saturation,
            // +20 % contrast against the Bevy default.
            Self::Vivid => LookParams {
                tonemapping: Tonemapping::SomewhatBoringDisplayTransform,
                post_saturation: 1.1,
                contrast: 1.0,
                ambient_scale: 0.5,
            },
            Self::Natural | Self::BevyDefault => LookParams {
                tonemapping: Tonemapping::default(),
                post_saturation: 1.0,
                contrast: 1.0,
                ambient_scale: 1.0,
            },
        }
    }
}
