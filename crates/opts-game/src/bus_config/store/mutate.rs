//! Changing the saved configurations: saving, deleting, renaming, duplicating, marking a favourite and touching.

use super::*;

impl BusConfigurations {
    /// Saves `config`: a configuration with the same name (ignoring case) is replaced and keeps
    /// its id, creation time, notes and favourite mark; otherwise it is added. Returns the id
    /// and whether one was replaced.
    pub fn save(&mut self, mut config: BusConfiguration) -> (String, bool) {
        if let Some(at) = self
            .configurations
            .iter()
            .position(|c| same_name(&c.name, &config.name))
        {
            let old = &self.configurations[at];
            config.id = old.id.clone();
            config.created = old.created;
            if config.notes.is_empty() {
                config.notes = old.notes.clone();
            }
            config.favourite = old.favourite;
            let id = config.id.clone();
            self.configurations[at] = config;
            return (id, true);
        }
        config.id = self.fresh_id(&config.name);
        let id = config.id.clone();
        self.configurations.push(config);
        (id, false)
    }

    /// Removes a configuration; whether there was one.
    pub fn delete(&mut self, key: &str) -> bool {
        match self.position(key) {
            Some(at) => {
                self.configurations.remove(at);
                true
            }
            None => false,
        }
    }

    /// Renames a configuration. `false` when it is missing or the name is empty or taken by
    /// another one.
    pub fn rename(&mut self, key: &str, name: &str) -> bool {
        let name = name.trim();
        let Some(at) = self.position(key) else {
            return false;
        };
        let clash = self
            .configurations
            .iter()
            .enumerate()
            .any(|(i, c)| i != at && same_name(&c.name, name));
        if name.is_empty() || clash {
            return false;
        }
        self.configurations[at].name = name.to_string();
        true
    }

    /// Copies a configuration under `name`; the id of the copy, `None` when the original is
    /// missing or the name is empty or taken.
    pub fn duplicate(&mut self, key: &str, name: &str, now: i64) -> Option<String> {
        let name = name.trim();
        if name.is_empty() || self.find(name).is_some() {
            return None;
        }
        let mut copy = self.find(key)?.clone();
        copy.name = name.to_string();
        copy.favourite = false;
        copy.created = now;
        copy.last_used = now;
        copy.id = self.fresh_id(name);
        let id = copy.id.clone();
        self.configurations.push(copy);
        Some(id)
    }

    /// Sets the favourite mark; `false` when the configuration is missing.
    pub fn set_favourite(&mut self, key: &str, favourite: bool) -> bool {
        match self.position(key) {
            Some(at) => {
                self.configurations[at].favourite = favourite;
                true
            }
            None => false,
        }
    }

    /// Stamps a configuration as used now.
    pub fn touch(&mut self, key: &str, now: i64) {
        if let Some(at) = self.position(key) {
            self.configurations[at].last_used = now;
        }
    }
}
