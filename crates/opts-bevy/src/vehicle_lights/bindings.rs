//! The bindings of a vehicle's lights to its script variables: a light map and an alpha scale.

use super::*;

/// A mesh part whose material has a light map driven by a script variable.
///
/// The part's material may be shared with other vehicles of the same model; it is replaced by a
/// private copy the first time the owner runs scripts (parked vehicles never pay for the copy).
#[derive(Component, Debug, Clone)]
pub struct LightMapBinding {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// Brightness variable.
    pub variable: String,
    /// The material is this vehicle's own copy.
    pub private: bool,
    /// Last applied factor (`NaN` before the first update).
    pub applied: f32,
}

impl LightMapBinding {
    /// Binds `variable` for the vehicle `owner`.
    #[must_use]
    pub fn new(owner: Entity, variable: impl Into<String>) -> Self {
        Self {
            owner,
            variable: variable.into(),
            private: false,
            applied: f32::NAN,
        }
    }
}

/// A mesh part whose material alpha follows a script variable (`[alphascale]`): window glass
/// scaled by `Envir_Brightness`, panes switched by `szyby`, condensation by `Beschlag_*`.
///
/// The part is visible while the variable is above [`ALPHA_VISIBLE_MIN`]; its material (a private
/// copy, made on first use) gets `base alpha × variable` unless the material is a cutout, which
/// only switches. A hidden part costs nothing.
#[derive(Component, Debug, Clone)]
pub struct AlphaScaleBinding {
    /// The vehicle root that owns the script engine.
    pub owner: Entity,
    /// The scaling variable.
    pub variable: String,
    /// The material is this vehicle's own copy.
    pub private: bool,
    /// Alpha of the material before scaling (`NaN` until the copy is made).
    pub base_alpha: f32,
    /// Last applied factor (`NaN` before the first update).
    pub applied: f32,
}

/// A part whose alpha scale is below this is not drawn.
pub const ALPHA_VISIBLE_MIN: f32 = 0.003;

impl AlphaScaleBinding {
    /// Binds `variable` for the vehicle `owner`.
    #[must_use]
    pub fn new(owner: Entity, variable: impl Into<String>) -> Self {
        Self {
            owner,
            variable: variable.into(),
            private: false,
            base_alpha: f32::NAN,
            applied: f32::NAN,
        }
    }
}
