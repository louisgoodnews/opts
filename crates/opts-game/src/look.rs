//! The colour *look* of the 3D view: tonemapper, post-saturation, contrast and ambient scale.
//!
//! A [`Look`] is a user setting (settings screen, `--look`, control protocol `render.look`) that
//! maps to [`LookParams`]. The [`ActiveLook`] resource holds the look in effect; [`LookPlugin`]
//! applies its parameters live to every `Camera3d` ([`apply_look_to_cameras`]) and the sky system
//! reads [`LookParams::ambient_scale`]. Provisional values, see M-0060 step 1 (physical calibration
//! follows in step 2): *Vivid* is the default, *Bevy default* reproduces the behaviour before the
//! looks existed.
//!
//! Spec: `docs/src/architecture/texture-quality.md` §3.2 and `docs/src/roadmap/phase-8/M-0060.md`.

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

/// The look in effect. Starts as the setting (or the `--look` override of this run) and changes
/// live through the settings screen and the control protocol.
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveLook(pub Look);

/// Applies the render options at start: the stored settings, overridden for this run by
/// `--texture-filter` / `--look` (the overrides are not persisted).
pub fn apply_startup_render_options(
    app: &mut bevy::prelude::App,
    settings: &crate::settings::GameSettings,
    args: &opts_cli::CliArgs,
) {
    let filtering = match args.texture_filter.as_deref() {
        Some(text) => opts_bevy::TextureFiltering::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --texture-filter (off, 4, 8, 16); using the setting"
            );
            settings.texture_filtering
        }),
        None => settings.texture_filtering,
    };
    opts_bevy::set_texture_filtering(filtering);
    let look = match args.look.as_deref() {
        Some(text) => Look::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --look (vivid, natural, bevy); using the setting"
            );
            settings.look
        }),
        None => settings.look,
    };
    tracing::info!(
        texture_filtering = filtering.key(),
        look = look.key(),
        "render options"
    );
    app.insert_resource(ActiveLook(look));
}
