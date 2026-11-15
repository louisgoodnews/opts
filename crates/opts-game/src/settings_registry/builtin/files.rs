//! The files and catalogue settings: the OMSI folder, the discovery interval and the recent map.

use crate::settings_registry::descriptor::{
    Apply, Category as C, SettingDescriptor as D, SettingKind as K,
};

/// The OMSI folder, the catalogue interval and the recent map.
pub(super) fn files() -> Vec<D> {
    vec![
        D::new(
            "omsi_path",
            K::Path,
            "",
            C::Content,
            10,
            "set-omsi-path",
            "set-omsi-path-desc",
        )
        .keywords(&["folder", "installation", "directory", "steam"])
        .apply(Apply::OnRestart),
        D::new(
            "discovery_interval_days",
            K::Int {
                min: 1,
                max: 90,
                step: 1,
            },
            "1",
            C::Content,
            20,
            "set-discovery-interval",
            "set-discovery-interval-desc",
        )
        .keywords(&["catalogue", "scan", "refresh"]),
        D::new(
            "content.rescan",
            K::Action,
            "",
            C::Content,
            30,
            "set-rescan",
            "set-rescan-desc",
        )
        .keywords(&["catalogue", "discovery", "refresh", "scan"])
        .since("0.2"),
        D::new(
            "recent_map_path",
            K::Path,
            "",
            C::Content,
            90,
            "set-recent-map",
            "set-recent-map-desc",
        )
        .hidden(),
    ]
}
