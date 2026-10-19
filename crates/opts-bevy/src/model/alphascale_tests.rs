//! Alpha-scale tests of the model loader.

use super::TextureOverride;

fn over(alpha: Option<u32>, trans: Option<&str>, scaled: bool) -> TextureOverride {
    over_with(alpha, trans, scaled, "Dirt_Norm")
}

fn over_with(alpha: Option<u32>, trans: Option<&str>, scaled: bool, name: &str) -> TextureOverride {
    TextureOverride {
        slot: 0,
        texture: "t.tga".into(),
        alpha,
        alpha_variable: scaled.then(|| name.to_owned()),
        trans_map: trans.map(str::to_owned),
        light_map: None,
        image: None,
    }
}

/// MAN SD83: the window glass `Fenster.tga` is `[matl_alpha] 2` scaled by `Envir_Brightness`;
/// it follows the variable at run time instead of being dropped. Weather overlays stay out.
#[test]
fn glass_scaled_by_a_live_variable_is_built_and_follows_it() {
    let glass = over_with(Some(2), None, true, "Envir_Brightness");
    assert!(!glass.is_variable_hidden());
    assert_eq!(glass.runtime_alpha_variable(), Some("Envir_Brightness"));
    for weather in ["Rain_Window_Norm_Wetness", "Dirt_Wiped", "Precip_Mirror"] {
        let o = over_with(Some(2), None, true, weather);
        assert!(o.is_variable_hidden(), "{weather}");
        assert_eq!(o.runtime_alpha_variable(), None);
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

/// MI-0029: `[alphascale] Envir_Brightness` on an opaque base material (380 materials of the
/// corpus: Crealis bodies, the Chicago skyline) must not hide the surface; overlays that use
/// alpha (rain, dirt, condensation: 43,968 materials) stay hidden by their variable.
#[test]
fn only_alpha_overlays_are_hidden_by_their_variable() {
    assert!(
        !over(None, None, true).is_variable_hidden(),
        "opaque body stays"
    );
    assert!(
        !over(Some(0), None, true).is_variable_hidden(),
        "mode 0 is opaque"
    );
    assert!(
        over(Some(2), None, true).is_variable_hidden(),
        "blended overlay"
    );
    assert!(
        over(Some(1), None, true).is_variable_hidden(),
        "cutout overlay"
    );
    assert!(
        over(None, Some("mask.bmp"), true).is_variable_hidden(),
        "transmap overlay"
    );
    assert!(
        !over(Some(2), None, false).is_variable_hidden(),
        "no variable: visible"
    );
}
