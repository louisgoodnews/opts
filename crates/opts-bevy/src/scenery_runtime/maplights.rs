//! `[maplight]` ground lights of scenery objects (street lamps: 1,771 on Berlin-Spandau).
//!
//! Every `[maplight]` becomes a [`MapLight`] marker entity under its object; no Bevy light is
//! created per lamp. A fixed pool of shadowless point lights ([`MapLightPool`]) is handed every
//! [`MapLightSettings::interval_s`] to the nearest lamps within [`MapLightSettings::radius_m`] of
//! the camera (with hysteresis, so a lamp at the edge does not flicker in and out), switched by
//! the night switch (`NightlightA`, [`SceneryClock::night`]) and faded in and out.
//!
//! Photometry: a `[maplight]` gives the radius `r` at which the light is at full intensity, then
//! falls off with the square of the distance (the comment in the stock lamp objects). With a full
//! illuminance `E_full` at `r`, the point light's luminous power is `4π·r²·E_full` lumens; its range
//! is `8·r`. `E_full` is a tuning value ([`MapLightSettings::full_lux`]).

#[cfg(test)]
use bevy::prelude::*;

mod component;
mod photometry;
mod pool;
mod settings;
mod slot_light;
mod system;

pub use component::{spawn_map_lights, MapLight};
pub use photometry::{choose_nearest, map_light_lumens, map_light_range};
pub use pool::{assign, MapLightPool, PoolSlot};
pub use settings::MapLightSettings;
pub use system::map_light_pool_system;

#[cfg(test)]
mod tests;
