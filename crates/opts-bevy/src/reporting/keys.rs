//! Message keys of the report dialog. The tables live with the application (`opts-game`), which
//! merges them into the localisation; the English text next to each key is the fallback used
//! when no table is loaded (headless tests).

/// Dialog title.
pub const TITLE: (&str, &str) = ("report-title", "Report an issue");
/// Label of the text area.
pub const DESCRIBE: (&str, &str) = ("report-describe", "Describe the issue:");
/// Placeholder of the text area.
pub const PLACEHOLDER: (&str, &str) = ("report-placeholder", "What happened? What did you expect?");
/// Caption of the screenshot preview.
pub const PREVIEW: (&str, &str) = (
    "report-preview",
    "This screenshot is saved with your report.",
);
/// Save button.
pub const SAVE: (&str, &str) = ("report-save", "Save report");
/// Cancel button.
pub const CANCEL: (&str, &str) = ("report-cancel", "Cancel");
/// Inline error: nowhere to save.
pub const NO_TARGET: (&str, &str) = (
    "report-error-no-target",
    "The report cannot be saved: there is no reports folder or no screenshot.",
);
/// Toast after a save (variable `path`).
pub const SAVED: (&str, &str) = ("report-saved", "Report saved to { $path }");
/// Inline error and toast after a failed save (variable `error`).
pub const FAILED: (&str, &str) = ("report-failed", "The report could not be saved: { $error }");

/// Toast and nothing else to show: the screenshot could not be taken, so no dialog opens.
pub const CAPTURE_FAILED: (&str, &str) = (
    "report-capture-failed",
    "The report could not be started: the screenshot failed.",
);

/// Every key, for the completeness test of the application.
pub const ALL: &[(&str, &str)] = &[
    TITLE,
    DESCRIBE,
    PLACEHOLDER,
    PREVIEW,
    SAVE,
    CANCEL,
    NO_TARGET,
    SAVED,
    FAILED,
];
