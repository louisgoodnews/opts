//! Reading and writing the viewpoint file: parsing its sections with the load report, the JSON text, and the load and save of the file.

use super::*;

impl ViewpointStore {
    /// Parses file text; never fails. Invalid entries are dropped and listed in the report.
    #[must_use]
    pub fn parse(text: &str) -> (Self, LoadReport) {
        let mut store = Self::default();
        let mut report = LoadReport::default();
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(text) else {
            report.unreadable = true;
            report.problems.push("not a JSON object".to_owned());
            return (store, report);
        };
        match root.get("version").and_then(Value::as_u64) {
            Some(v) if v <= FILE_VERSION => {}
            other => {
                report.problems.push(format!(
                    "unsupported version {other:?}; its entries are ignored"
                ));
                return (store, report);
            }
        }
        store.vehicles = Self::parse_section(root.get("vehicles"), "vehicle", true, &mut report);
        store.models = Self::parse_section(root.get("models"), "model", false, &mut report);
        (store, report)
    }

    /// The file text.
    #[must_use]
    pub fn to_json(&self) -> String {
        let section = |section: &Section| -> serde_json::Map<String, Value> {
            section
                .iter()
                .map(|(key, cams)| {
                    let cams: serde_json::Map<String, Value> = cams
                        .iter()
                        .map(|(i, vp)| (i.to_string(), vp_json(vp)))
                        .collect();
                    (key.clone(), json!({ "cameras": cams }))
                })
                .collect()
        };
        serde_json::to_string_pretty(&json!({
            "version": FILE_VERSION,
            "vehicles": section(&self.vehicles),
            "models": section(&self.models),
        }))
        .unwrap_or_default()
    }

    /// Reads `path`. A missing file is an empty store; an unusable one is moved aside.
    #[must_use]
    pub fn load_from(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let (store, report) = Self::parse(&text);
        for p in &report.problems {
            tracing::warn!(file = %path.display(), "viewpoints: {p}");
        }
        if report.unreadable {
            let bad = path.with_extension("json.bad");
            match std::fs::rename(path, &bad) {
                Ok(()) => {
                    tracing::warn!(moved_to = %bad.display(), "viewpoints: the file was unusable")
                }
                Err(e) => tracing::warn!("viewpoints: cannot move the unusable file aside: {e}"),
            }
        }
        store
    }

    /// Writes the store to `path` (through a temporary file, so a crash never leaves half a file).
    ///
    /// # Errors
    /// The I/O error when the file cannot be written.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(&tmp, path)
    }
}
