//! system values, variables and identifiers of the engine.

use super::*;

impl ScriptEngine {

    /// Sets a system value read by `(L.S.name)`.
    pub fn set_system(&mut self, name: &str, value: f32) {
        let id = self.system_id(name);
        self.set_system_id(id, value);
    }

    /// Index of system value `name` for [`Self::set_system_id`] (hosts that set the same values
    /// every frame resolve the names once).
    pub fn system_id(&mut self, name: &str) -> u32 {
        self.program.names.sys.intern(name)
    }

    /// Sets the system value with index `id` (from [`Self::system_id`]).
    pub fn set_system_id(&mut self, id: u32, value: f32) {
        let id = id as usize;
        if self.sys.len() <= id {
            self.sys.resize(id + 1, 0.0);
        }
        self.sys[id] = value;
    }

    /// Index of float variable `name` for [`Self::get_id`] / [`Self::set_id`].
    pub fn variable_id(&mut self, name: &str) -> u32 {
        self.program.names.vars.intern(name)
    }

    /// Reads the float variable with index `id` (from [`Self::variable_id`]).
    #[must_use]
    pub fn get_id(&self, id: u32) -> f32 {
        self.vars.float(id)
    }

    /// Writes the float variable with index `id` (from [`Self::variable_id`]).
    pub fn set_id(&mut self, id: u32, value: f32) {
        self.vars.set_float(id, value);
    }

    /// A fresh engine running the same compiled program with the same constants and variable
    /// declarations, but its own (zeroed) variables, events and generator state. Hosts that
    /// place one script set many times compile it once and fork it per instance.
    #[must_use]
    pub fn fork(&self) -> Self {
        let mut e = Self::new(self.program.clone());
        e.consts = self.consts.clone();
        e.declared = self.declared.clone();
        e.rng = self.rng;
        e
    }

    /// The value of scalar constant `name` (a const file's `[const]`), `None` when there is none.
    #[must_use]
    pub fn const_scalar(&self, name: &str) -> Option<f32> {
        self.consts.scalar(name)
    }

    /// Reads float variable `name` (`0` if the scripts never touched it).
    #[must_use]
    pub fn get(&self, name: &str) -> f32 {
        self.program
            .names
            .vars
            .get(name)
            .map_or(0.0, |id| self.vars.float(id))
    }

    /// Writes float variable `name` (host inputs such as pedal positions).
    pub fn set(&mut self, name: &str, value: f32) {
        let id = self.program.names.vars.intern(name);
        self.vars.set_float(id, value);
    }

    /// Reads string variable `name`.
    #[must_use]
    pub fn get_string(&self, name: &str) -> &str {
        self.program
            .names
            .strvars
            .get(name)
            .map_or("", |id| self.vars.string(id))
    }

    /// Writes string variable `name`.
    pub fn set_string(&mut self, name: &str, value: &str) {
        let id = self.program.names.strvars.intern(name);
        self.vars.set_string(id, value.to_owned());
    }

}
