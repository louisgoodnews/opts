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
         --- Description ---\n\n{}\n",
        meta.timestamp,
        meta.version,
        meta.git,
        meta.platform,
        meta.map,
        meta.camera,
        meta.vehicle,
        meta.focused_ui,
        desc
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
