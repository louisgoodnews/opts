//! The `.report` file text (see `docs/src/formats/text/report-format.md`).

use super::state::ReportContext;

/// Fields written above the description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportMeta {
    /// RFC 3339 local timestamp with offset.
    pub timestamp: String,
    /// Application version.
    pub version: String,
    /// Short git hash.
    pub git: String,
    /// `std::env::consts::OS`.
    pub platform: String,
    /// Loaded map.
    pub map: String,
    /// Camera mode.
    pub camera: String,
    /// Player vehicle.
    pub vehicle: String,
    /// Widget that had focus when the report was sent.
    pub focused_ui: String,
    /// Hardware profile text, present only when the user allowed it.
    pub hardware: Option<String>,
}

fn or_none(v: &Option<String>) -> String {
    v.as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("(none)")
        .to_owned()
}

impl ReportMeta {
    /// Builds the header from the application context; unknown fields become `(none)`.
    #[must_use]
    pub fn from_context(ctx: &ReportContext, timestamp: String, focused_ui: Option<&str>) -> Self {
        Self {
            timestamp,
            version: or_none(&ctx.version),
            git: or_none(&ctx.git),
            platform: std::env::consts::OS.to_owned(),
            map: or_none(&ctx.map),
            camera: or_none(&ctx.camera),
            vehicle: or_none(&ctx.vehicle),
            focused_ui: focused_ui.unwrap_or("(none)").to_owned(),
            hardware: ctx.hardware.clone().filter(|h| !h.is_empty()),
        }
    }
}

/// Renders the report: metadata header, then the description verbatim.
///
/// The output is UTF-8 with `\n` line endings (the description's `\r\n` / `\r` are normalised;
/// everything else is left untouched).
#[must_use]
pub fn format_report(meta: &ReportMeta, description: &str) -> String {
    let desc = description.replace("\r\n", "\n").replace('\r', "\n");
    let hardware = meta.hardware.as_ref().map_or_else(String::new, |h| {
        format!(
            "\n--- Hardware profile (included because you allowed it in Settings > Advanced) ---\n\n{}\n",
            h.trim_end()
        )
    });
    format!(
        "OPTS Report\n===========\n\n\
         Timestamp:   {}\n\
         Version:     {}\n\
         Git:         {}\n\
         Platform:    {}\n\
         Map:         {}\n\
         Camera:      {}\n\
         Vehicle:     {}\n\
         Focused UI:  {}\n\n\
         --- Description ---\n\n{}\n{}",
        meta.timestamp,
        meta.version,
        meta.git,
        meta.platform,
        meta.map,
        meta.camera,
        meta.vehicle,
        meta.focused_ui,
        desc,
        hardware
    )
}

/// `YYYY-MM-DD_HH-MM-SS` file stem and RFC 3339 timestamp for the current local time.
#[must_use]
pub fn now_stamp() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format("%Y-%m-%d_%H-%M-%S").to_string(),
        now.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
    )
}

#[cfg(test)]
mod hardware_tests {
    use super::*;

    #[test]
    fn hardware_section_only_when_the_context_carries_it() {
        let mut ctx = ReportContext::default();
        let plain = format_report(&ReportMeta::from_context(&ctx, "t".into(), None), "text");
        assert!(!plain.contains("Hardware profile"));
        ctx.hardware = Some("cpu.logical_cores  8\n".into());
        let with = format_report(&ReportMeta::from_context(&ctx, "t".into(), None), "text");
        assert!(with.contains("--- Hardware profile"));
        assert!(with.ends_with("cpu.logical_cores  8\n"));
        assert!(with.contains("text\n\n--- Hardware"));
    }
}
