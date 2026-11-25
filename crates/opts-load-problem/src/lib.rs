//! One problem of a load and how much it matters: the severity table and the problem constructors.

/// How much a problem matters for the drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// The map, a map tile, or the player's vehicle did not load: the screen waits for the user.
    Blocking,
    /// Individual assets (textures, scenery meshes, fonts, text sources) are missing: the drive
    /// starts by itself and the problems are announced afterwards.
    Minor,
}

/// The one place that says which kind of problem blocks the screen.
pub fn severity_of(key: &str) -> Severity {
    match key {
        "load-problem-objects" | "load-problem-files" => Severity::Minor,
        // load-problem-map, -vehicle, -no-model, -tiles and anything unknown block.
        _ => Severity::Blocking,
    }
}

/// One thing that did not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadProblem {
    /// Nothing usable came out of it (the load failed).
    pub fatal: bool,
    /// Message key of the sentence that says what happened.
    pub key: &'static str,
    /// Variables of that sentence (`name`, `count`, `total`).
    pub args: Vec<(String, String)>,
    /// Message key of the cause in plain words, if one is known.
    pub cause_key: Option<&'static str>,
    /// Paths, error texts and codes for the details view.
    pub detail: String,
}

impl LoadProblem {
    /// How much this problem matters.
    pub fn severity(&self) -> Severity {
        severity_of(self.key)
    }

    /// A problem with one variable.
    pub fn new(fatal: bool, key: &'static str, name: &str, detail: impl Into<String>) -> Self {
        Self {
            fatal,
            key,
            args: vec![("name".to_string(), name.to_string())], // l10n-ok: variable name
            cause_key: None,
            detail: detail.into(),
        }
    }

    /// Adds the plain-language cause.
    #[must_use]
    pub fn because(mut self, cause_key: &'static str) -> Self {
        self.cause_key = Some(cause_key);
        self
    }

    /// A counted problem ("3 of 9 map tiles are missing").
    pub fn counted(
        key: &'static str,
        count: usize,
        total: usize,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            fatal: false,
            key,
            args: vec![
                ("count".to_string(), count.to_string()), // l10n-ok: variable name
                ("total".to_string(), total.to_string()), // l10n-ok: variable name
            ],
            cause_key: None,
            detail: detail.into(),
        }
    }
}
