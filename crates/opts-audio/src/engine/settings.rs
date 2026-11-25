//! The volume settings of the sound engine: the percent settings and the bus gains they give.

use super::*;

/// The volume settings (percent) and limits; the game keeps it in step with its settings.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct AudioSettings {
    /// Sound on or off altogether.
    pub enabled: bool,
    /// Master volume, 0 to 100.
    pub master: f32,
    /// Effects bus, 0 to 100.
    pub effects: f32,
    /// Ambient bus, 0 to 100.
    pub ambient: f32,
    /// Vehicles bus, 0 to 100.
    pub vehicles: f32,
    /// Interface bus, 0 to 100.
    pub ui: f32,
    /// Most voices playing at once (the loudest win).
    pub max_voices: usize,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master: 80.0,
            effects: 80.0,
            ambient: 60.0,
            vehicles: 80.0,
            ui: 60.0,
            max_voices: 32,
        }
    }
}

impl AudioSettings {
    /// The linear gain of a bus including the master volume.
    #[must_use]
    pub fn bus_gain(&self, bus: SoundBus) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        let percent = match bus {
            SoundBus::Effects => self.effects,
            SoundBus::Ambient => self.ambient,
            SoundBus::Vehicles => self.vehicles,
            SoundBus::Ui => self.ui,
        };
        percent_to_gain(self.master) * percent_to_gain(percent)
    }
}
