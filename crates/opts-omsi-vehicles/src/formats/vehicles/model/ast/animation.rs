//! Variable-driven animations of a vehicle model (`[newanim]`).

use glam::Vec3;

/// Variable-driven animation action (translation or rotation).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimAction {
    /// Script variable name that drives this animation transform.
    pub variable: String,
    /// Scaling multiplier applied to the variable value.
    pub factor: f32,
}

/// Category of an animation action.
#[derive(Debug, Clone, PartialEq)]
pub enum AnimType {
    /// Rotation transform around the origin axis.
    Rotation(AnimAction),
    /// Translation transform along the origin axis.
    Translation(AnimAction),
}

/// Animation definition for moving and rotating vehicle meshes (`[newanim]`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelAnim {
    /// Translation offset of the animation pivot origin in local mesh space.
    pub origin_trans: Option<Vec3>,
    /// Rotation angles of the animation pivot origin in degrees (X, Y, Z).
    pub origin_rot: Vec3,
    /// Indicates whether the animation origin is derived directly from the mesh pivot.
    pub origin_from_mesh: bool,
    /// Variable-driven rotation action.
    pub anim_rot: Option<AnimAction>,
    /// Variable-driven translation action.
    pub anim_trans: Option<AnimAction>,
    /// Damping or delay parameter for motion smoothing.
    pub delay: Option<f32>,
    /// Maximum animation speed limit.
    pub maxspeed: Option<f32>,
    /// Animation start offset.
    pub offset: Option<f32>,
    /// General animation driving variable name (`anim_val`).
    pub anim_val: Option<String>,
    /// Maximum value bound for the driving variable (`anim_maxval`).
    pub anim_maxval: Option<f32>,
    /// Unhandled sub-properties preserved for round-trip fidelity.
    pub unknown_properties: Vec<(String, Vec<String>)>,
}

impl ModelAnim {
    /// Returns true if this animation includes a rotation action.
    #[must_use]
    pub fn is_rotation(&self) -> bool {
        self.anim_rot.is_some()
    }

    /// Returns true if this animation includes a translation action.
    #[must_use]
    pub fn is_translation(&self) -> bool {
        self.anim_trans.is_some()
    }

    /// Returns the primary driving variable name if present.
    #[must_use]
    pub fn driver_variable(&self) -> Option<&str> {
        if let Some(ref rot) = self.anim_rot {
            Some(&rot.variable)
        } else if let Some(ref trans) = self.anim_trans {
            Some(&trans.variable)
        } else {
            self.anim_val.as_deref()
        }
    }
}
