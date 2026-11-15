//! The sky side of the session environment: the sun, the moon and the attenuation.

use super::*;

impl SessionEnvironment {
    /// ISO day of the week, 1 = Monday … 7 = Sunday.
    #[must_use]
    pub fn weekday(&self) -> u32 {
        self.date.weekday()
    }

    /// The `[set_day_of_week]` bit of today.
    #[must_use]
    pub fn day_bit(&self) -> u8 {
        self.date.day_bit()
    }

    /// Sun elevation in degrees.
    #[must_use]
    pub fn sun_alt_deg(&self) -> f32 {
        self.sun_alt_deg
    }

    /// Sun azimuth in degrees, clockwise from north.
    #[must_use]
    pub fn sun_az_deg(&self) -> f32 {
        self.sun_az_deg
    }

    /// Whether it is night (sun below [`NIGHT_SUN_ALT_DEG`]).
    #[must_use]
    pub fn night(&self) -> bool {
        self.sun_alt_deg < NIGHT_SUN_ALT_DEG
    }

    /// The moon (position, phase, bright limb) for the session's date and time, refreshed with
    /// the sun.
    #[must_use]
    pub fn moon(&self) -> &MoonState {
        &self.moon
    }

    /// What the weather does to every light ([`light_attenuation`]).
    #[must_use]
    pub fn light_attenuation(&self) -> LightAttenuation {
        light_attenuation(&WeatherSky::from_conditions(&self.conditions))
    }

    /// Multiplier for the sun's illuminance under the weather: the `direct` part of
    /// [`Self::light_attenuation`].
    #[must_use]
    pub fn attenuation(&self) -> f32 {
        self.light_attenuation().direct
    }

    /// Multiplier for the ambient light under the weather ([`crate::weather::light_scales`]).
    #[must_use]
    pub fn ambient_attenuation(&self) -> f32 {
        self.light_attenuation().ambient
    }

    /// Outside temperature in °C.
    #[must_use]
    pub fn temperature_c(&self) -> f32 {
        self.conditions.temperature_c
    }
}
