//! Smoke emitters of a vehicle model (`[smoke]`): exhaust, tyre spray and coolant puffs.

use glam::Vec3;

/// A particle emitter of the vehicle (`[smoke]`). Only the position, the direction and the two
/// variable names are read; the size, alpha and colour values are kept for later use.
#[derive(Debug, Clone, PartialEq)]
pub struct SmokeEmitter {
    /// Emitter position in vehicle space (the first three values).
    pub pos: Vec3,
    /// Direction the puffs move in (values 4 to 6).
    pub dir: Vec3,
    /// Script variable that triggers the emission (value 7), if given.
    pub variable: Option<String>,
    /// Script variable that sets the emission frequency (value 9), if given.
    pub freq_variable: Option<String>,
}
