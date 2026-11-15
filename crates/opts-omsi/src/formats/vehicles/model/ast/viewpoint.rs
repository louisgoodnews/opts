//! Viewpoint visibility flags of a mesh (`[mesh]` camera bitmask).

/// Viewpoint visibility bitmask controlling whether a mesh is visible in different camera views.
///
/// Bit values defined by OMSI:
/// - `0`: Always visible in all views.
/// - `1`: Visible only from outside the vehicle.
/// - `2`: Visible only from inside the passenger compartment/cab.
/// - `4`: Visible only when vehicle is driven as an AI bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewpointFlags(pub u32);

impl ViewpointFlags {
    /// Always visible in every camera viewpoint.
    pub const ALWAYS: Self = Self(0);
    /// Visible from outside viewpoints.
    pub const OUTSIDE: Self = Self(1);
    /// Visible from interior cab and passenger viewpoints.
    pub const INSIDE: Self = Self(2);
    /// Visible when the vehicle is an AI-controlled bus.
    pub const AI_BUS: Self = Self(4);

    /// Creates a new viewpoint flags mask.
    #[must_use]
    pub const fn new(mask: u32) -> Self {
        Self(mask)
    }

    /// Returns the raw bitmask value.
    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Checks if the mesh is configured to be unconditionally visible.
    #[must_use]
    pub const fn is_always(&self) -> bool {
        self.0 == 0
    }

    /// Checks if the mesh is visible from external camera views.
    #[must_use]
    pub const fn is_outside(&self) -> bool {
        self.is_always() || (self.0 & 1) != 0
    }

    /// Checks if the mesh is visible from interior passenger and driver views.
    #[must_use]
    pub const fn is_inside(&self) -> bool {
        self.is_always() || (self.0 & 2) != 0
    }

    /// Checks if the mesh is visible when the vehicle is controlled by AI.
    #[must_use]
    pub const fn is_ai(&self) -> bool {
        self.is_always() || (self.0 & 4) != 0
    }
}
