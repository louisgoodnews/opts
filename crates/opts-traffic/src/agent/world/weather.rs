//! The weather of the traffic world (M-0117): the road grip and the visibility every driver sees.

use super::*;
use crate::weather::{weather_adjusted, WeatherState};

impl TrafficWorld {
    /// Sets the grip factor of the road (`1.0`: dry asphalt) and the visibility (m, infinite in
    /// clear air) for all drivers. Takes effect from the next step.
    pub fn set_weather(&mut self, grip: f64, visibility_m: f64) {
        self.weather = WeatherState { grip, visibility_m };
    }

    /// The weather the drivers see.
    #[must_use]
    pub fn weather(&self) -> WeatherState {
        self.weather
    }

    /// The parameters `params` of a driver with the weather applied.
    #[must_use]
    pub(crate) fn driver(&self, params: &IdmParams) -> IdmParams {
        weather_adjusted(params, self.weather.grip, self.weather.visibility_m)
    }
}
