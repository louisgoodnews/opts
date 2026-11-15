//! Graphics settings: how far shadows, lights and detail reach.

use super::*;

/// The sun's shadows: how far they reach and how far small parts cast (M-0029).
pub(super) fn shadows() -> Vec<D> {
    vec![
        D::new(
            "graphics.shadow_distance",
            K::Int {
                min: 50,
                max: 2000,
                step: 50,
            },
            "300",
            C::Graphics,
            61,
            "set-shadow-distance",
            "set-shadow-distance-desc",
        )
        .keywords(&["shadows", "cascade", "sun", "distance"])
        .since("0.5"),
        D::new(
            "graphics.shadow_detail",
            K::Int {
                min: 0,
                max: 400,
                step: 25,
            },
            "0",
            C::Graphics,
            64,
            "set-shadow-detail",
            "set-shadow-detail-desc",
        )
        .keywords(&["shadows", "small objects", "culling", "performance"])
        .since("0.5")
        // Measured without a gain (lod-culling.md §3.1): kept for the control protocol only.
        .hidden(),
    ]
}

/// The light and detail distances.
pub(super) fn detail() -> Vec<D> {
    vec![
        D::new(
            "graphics.light_distance",
            K::Int {
                min: 50,
                max: 2000,
                step: 50,
            },
            "350",
            C::Graphics,
            62,
            "set-light-distance",
            "set-light-distance-desc",
        )
        .keywords(&["lights", "lamps", "street lights", "night", "distance"])
        .since("0.5"),
        D::new(
            "graphics.detail_distance",
            K::Int {
                min: 25,
                max: 800,
                step: 25,
            },
            "200",
            C::Graphics,
            63,
            "set-detail-distance",
            "set-detail-distance-desc",
        )
        .keywords(&["lod", "detail", "scenery", "objects", "distance"])
        .since("0.5"),
    ]
}
