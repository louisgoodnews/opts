//! Validation and typed reading of the stored text of a setting.

use super::descriptor::{SettingDescriptor, SettingKind};

/// Why a value was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingError {
    /// No setting with this key.
    Unknown(String),
    /// The value does not fit the kind; the text says what is expected.
    Invalid(String),
    /// The setting is planned or needs a capability that is missing.
    Unavailable(String),
}

impl std::fmt::Display for SettingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(k) => write!(f, "unknown setting \"{k}\""),
            Self::Invalid(m) | Self::Unavailable(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for SettingError {}

/// A parsed value.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// A switch.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A decimal.
    Float(f64),
    /// Anything stored as text (enum value, path, time, binding).
    Text(String),
}

fn parse_hhmm(text: &str) -> Option<String> {
    let (h, m) = text.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().parse().ok()?);
    (h < 24 && m < 60).then(|| format!("{h:02}:{m:02}"))
}

/// The canonical stored text of `text` for `d`, or why it is invalid.
///
/// Numbers are range checked (not clamped: a protocol client must learn that it asked for too
/// much), enum values are matched case-insensitively and stored as the declared value.
pub fn normalize(d: &SettingDescriptor, text: &str) -> Result<String, SettingError> {
    let bad = |what: String| SettingError::Invalid(format!("{}: {what}", d.key));
    match d.kind {
        SettingKind::Bool => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok("true".into()),
            "false" | "off" | "no" | "0" => Ok("false".into()),
            _ => Err(bad("expected true or false".into())),
        },
        SettingKind::Enum(options) => options
            .iter()
            .find(|o| o.value.eq_ignore_ascii_case(text.trim()))
            .map(|o| o.value.to_string())
            .ok_or_else(|| {
                let list: Vec<&str> = options.iter().map(|o| o.value).collect();
                bad(format!("expected one of {}", list.join(", ")))
            }),
        SettingKind::Int { min, max, .. } => {
            let v: i64 = text
                .trim()
                .parse()
                .map_err(|_| bad(format!("expected a whole number from {min} to {max}")))?;
            if (min..=max).contains(&v) {
                Ok(v.to_string())
            } else {
                Err(bad(format!("expected a whole number from {min} to {max}")))
            }
        }
        SettingKind::Float { min, max, .. } => {
            let v: f64 = text
                .trim()
                .parse()
                .map_err(|_| bad(format!("expected a number from {min} to {max}")))?;
            if v.is_finite() && (min..=max).contains(&v) {
                Ok(v.to_string())
            } else {
                Err(bad(format!("expected a number from {min} to {max}")))
            }
        }
        SettingKind::Time => parse_hhmm(text).ok_or_else(|| bad("expected HH:MM".into())),
        SettingKind::Text => {
            if text.chars().count() > 256 {
                Err(bad("at most 256 characters".into()))
            } else {
                Ok(text.trim().to_string())
            }
        }
        SettingKind::Path | SettingKind::Binding => Ok(text.trim().to_string()),
        SettingKind::Action => Err(bad("this setting is an action and holds no value".into())),
    }
}

/// Parses already normalised (or stored) text into a typed value.
#[must_use]
pub fn parse(d: &SettingDescriptor, text: &str) -> SettingValue {
    match d.kind {
        SettingKind::Bool => SettingValue::Bool(text.trim() == "true"),
        SettingKind::Int { .. } => SettingValue::Int(text.trim().parse().unwrap_or(0)),
        SettingKind::Float { .. } => SettingValue::Float(text.trim().parse().unwrap_or(0.0)),
        _ => SettingValue::Text(text.to_string()),
    }
}
