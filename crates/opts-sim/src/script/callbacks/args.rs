//! The arguments a host handler receives and the results it returns.

/// Arguments of one call, in the order the signature names them (`floats[0]` is the first
/// argument, pushed first by the script).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallbackArgs {
    /// Float arguments.
    pub floats: Vec<f32>,
    /// String arguments.
    pub strings: Vec<String>,
}

impl CallbackArgs {
    /// Float argument `i`, or 0.
    #[must_use]
    pub fn f(&self, i: usize) -> f32 {
        self.floats.get(i).copied().unwrap_or(0.0)
    }

    /// Float argument `i` as a non-negative index.
    #[must_use]
    pub fn index(&self, i: usize) -> usize {
        self.f(i).max(0.0) as usize
    }

    /// String argument `i`, or empty.
    #[must_use]
    pub fn s(&self, i: usize) -> &str {
        self.strings.get(i).map_or("", String::as_str)
    }
}

/// Results of one call: pushed in order (the last float becomes the top of the stack).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallbackResult {
    /// Float results.
    pub floats: Vec<f32>,
    /// String results.
    pub strings: Vec<String>,
}

impl CallbackResult {
    /// No result (`GiveChangeCoin`, the canvas calls).
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// One float result.
    #[must_use]
    pub fn float(v: f32) -> Self {
        Self {
            floats: vec![v],
            strings: Vec::new(),
        }
    }

    /// One string result.
    #[must_use]
    pub fn string(s: impl Into<String>) -> Self {
        Self {
            floats: Vec::new(),
            strings: vec![s.into()],
        }
    }
}
