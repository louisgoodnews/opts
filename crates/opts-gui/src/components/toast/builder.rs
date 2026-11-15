//! The toast builder: the message, the duration, the action and the dedupe key.

use bevy::prelude::*;

use super::ToastSeverity;

/// Builder for constructing Material Design 3 toast notifications.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_toast(mut commands: Commands) {
///     ToastBuilder::new("Service dispatched successfully")
///         .with_duration(4.0)
///         .with_action("DISMISS")
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ToastBuilder {
    pub message: String,
    pub duration_secs: Option<f32>,
    pub action_label: Option<String>,
    /// Severity (colours, default duration).
    pub severity: ToastSeverity,
    /// Key for merging identical toasts.
    pub dedupe_key: Option<String>,
}

impl ToastBuilder {
    /// Creates a new toast builder with the specified message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            duration_secs: Some(4.0),
            action_label: None,
            severity: ToastSeverity::Info,
            dedupe_key: None,
        }
    }

    /// Sets the severity; the duration follows unless it was set explicitly afterwards.
    pub fn with_severity(mut self, severity: ToastSeverity) -> Self {
        self.severity = severity;
        self.duration_secs = Some(severity.default_seconds());
        self
    }

    /// Merges this toast with an identical live one (same key) instead of stacking a second.
    pub fn with_dedupe_key(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    /// Sets the display duration in seconds before automatic dismissal (or None for persistent).
    pub fn with_duration(mut self, seconds: f32) -> Self {
        self.duration_secs = Some(seconds);
        self
    }

    /// Makes the toast persistent until manually dismissed.
    pub fn persistent(mut self) -> Self {
        self.duration_secs = None;
        self
    }

    /// Sets an optional action button label (e.g. "UNDO" or "CLOSE").
    pub fn with_action(mut self, action_label: impl Into<String>) -> Self {
        self.action_label = Some(action_label.into());
        self
    }
}
