//! The `OPTS_REFLECT` research switches, read once from the environment.

use std::sync::OnceLock;

use bevy::prelude::Resource;

/// The parsed `OPTS_REFLECT` switches (research prototypes only).
#[derive(Debug, Clone, Default, Resource)]
pub struct ReflectionSwitches {
    /// Wet-surface parameter set.
    pub wet: bool,
    /// Screen-space reflections (deferred).
    pub ssr: bool,
}

impl ReflectionSwitches {
    /// Parses a switch list such as `wet,ssr`; unknown items (the former `env`, `glass`, `mirror`
    /// switches are normal features now) are ignored.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for item in text.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let key = item.split_once('=').map_or(item, |(k, _)| k);
            match key {
                "wet" => s.wet = true,
                "ssr" => s.ssr = true,
                _ => {}
            }
        }
        s
    }

    /// Reads `OPTS_REFLECT`; all off when unset.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(&std::env::var("OPTS_REFLECT").unwrap_or_default())
    }

    /// Whether any switch is on.
    #[must_use]
    pub fn any(&self) -> bool {
        self.wet || self.ssr
    }
}

static SWITCHES: OnceLock<ReflectionSwitches> = OnceLock::new();

/// The switches read from the environment once.
pub fn switches() -> &'static ReflectionSwitches {
    SWITCHES.get_or_init(ReflectionSwitches::from_env)
}
