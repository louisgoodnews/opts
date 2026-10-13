//! The settings of *Settings → Advanced → Hardware*: two actions and one switch (default off).

use crate::settings_registry::descriptor::{Category as C, SettingDescriptor as D, SettingKind as K};

/// The descriptors; the profile itself is shown below the rows by `view`.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![
        D::new(
            "advanced.hardware_redetect",
            K::Action,
            "",
            C::Advanced,
            30,
            "set-hw-redetect",
            "set-hw-redetect-desc",
        )
        .keywords(&["hardware", "gpu", "cpu", "detect", "probe", "benchmark"])
        .since("0.2"),
        D::new(
            "advanced.hardware_delete",
            K::Action,
            "",
            C::Advanced,
            40,
            "set-hw-delete",
            "set-hw-delete-desc",
        )
        .keywords(&["hardware", "privacy", "profile", "forget", "remove"])
        .since("0.2"),
        D::new(
            super::ALLOW_REPORTS_KEY,
            K::Bool,
            "false",
            C::Advanced,
            50,
            "set-hw-reports",
            "set-hw-reports-desc",
        )
        .keywords(&["hardware", "privacy", "report", "bug", "profile"])
        .since("0.2"),
    ]
}
