//! Alpha-scale tests of the model loader.

use super::TextureOverride;
use crate::model::{overlay_class, OverlayClass};

fn over(alpha: Option<u32>, trans: Option<&str>, scaled: bool) -> TextureOverride {
    over_with(alpha, trans, scaled, "Dirt_Norm")
}

fn over_with(alpha: Option<u32>, trans: Option<&str>, scaled: bool, name: &str) -> TextureOverride {
    TextureOverride {
        clamp: false,
        slot: 0,
        texture: "t.tga".into(),
        alpha,
        alpha_variable: scaled.then(|| name.to_owned()),
        trans_map: trans.map(str::to_owned),
        light_map: None,
        free_texture: None,
        night_map: None,
        image: None,
    }
}

/// MAN SD83: the window glass `Fenster.tga` is `[matl_alpha] 2` scaled by `Envir_Brightness`;
/// it follows the variable at run time instead of being dropped. Weather overlays are built and
/// follow their script variables too (M-0120): their alpha is texture alpha times the variable.
#[test]
fn glass_and_weather_overlays_are_built_and_follow_their_variable() {
    let glass = over_with(Some(2), None, true, "Envir_Brightness");
    assert_eq!(glass.runtime_alpha_variable(), Some("Envir_Brightness"));
    for weather in [
        "Rain_Window_Norm_Wetness",
        "Dirt_Wiped",
        "Beschlag_Norm",
        "Precip_Mirror",
    ] {
        let o = over_with(Some(2), None, true, weather);
        assert!(o.is_weather_overlay_part(), "{weather}");
        assert_eq!(o.runtime_alpha_variable(), Some(weather));
    }
    // Script-set variables (`szyby`, `para_front`) follow the scripts.
    assert_eq!(
        over_with(Some(2), None, true, "szyby").runtime_alpha_variable(),
        Some("szyby")
    );
    // An opaque material is not an overlay, whatever its variable.
    assert_eq!(
        over_with(None, None, true, "Envir_Brightness").runtime_alpha_variable(),
        None
    );
}

/// The weather classes: each prefix of the overlay variables maps to one class, others to none.
#[test]
fn overlay_classes_by_variable_prefix() {
    assert_eq!(overlay_class("Dirt_Norm"), Some(OverlayClass::Dirt));
    assert_eq!(overlay_class(" dirt_wiped "), Some(OverlayClass::Dirt));
    assert_eq!(
        overlay_class("Rain_Window_Norm_Wetness"),
        Some(OverlayClass::RainWindow)
    );
    assert_eq!(
        overlay_class("Beschlag_Front"),
        Some(OverlayClass::Condensation)
    );
    assert_eq!(
        overlay_class("Precip_Mirror"),
        Some(OverlayClass::Precipitation)
    );
    assert_eq!(overlay_class("Envir_Brightness"), None);
    assert_eq!(overlay_class("szyby"), None);
}

/// MI-0029: `[alphascale] Envir_Brightness` on an opaque base material (380 materials of the
/// corpus: Crealis bodies, the Chicago skyline) is not an overlay; alpha overlays (rain, dirt,
/// condensation: 43,968 materials) are.
#[test]
fn only_alpha_materials_are_overlays() {
    assert!(!over(None, None, true).is_alpha_overlay(), "opaque body");
    assert!(
        !over(Some(0), None, true).is_alpha_overlay(),
        "mode 0 is opaque"
    );
    assert!(
        over(Some(2), None, true).is_alpha_overlay(),
        "blended overlay"
    );
    assert!(
        over(Some(1), None, true).is_alpha_overlay(),
        "cutout overlay"
    );
    assert!(
        over(None, Some("mask.bmp"), true).is_alpha_overlay(),
        "transmap overlay"
    );
    assert!(
        !over(Some(2), None, false).is_weather_overlay_part(),
        "no variable"
    );
    assert!(
        over(Some(2), None, true).is_weather_overlay_part(),
        "Dirt_Norm overlay"
    );
}
