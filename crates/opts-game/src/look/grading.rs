//! The look's math: the daylight, the ambient scale, the colour grading of a look, and the
//! tonemapper's inverse for the clear colour of the HDR camera.

use bevy::render::view::{ColorGrading, Tonemapping};

use super::LookParams;

/// How much of the day's light is up: 0 at and below the horizon, 1 from 25 degrees of sun
/// elevation on (smooth in between).
#[must_use]
pub fn daylight(sun_elevation_deg: f32) -> f32 {
    let t = (sun_elevation_deg / 25.0).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The ambient scale of a look at the given sun elevation. A look darkens the ambient light to
/// deepen shadows in daylight; at dusk and night the base ambient stays (the ambient floor is what
/// keeps the scene readable), so the scale fades to 1 as the sun sets.
#[must_use]
pub fn effective_ambient_scale(look_scale: f32, sun_elevation_deg: f32) -> f32 {
    1.0 + (look_scale - 1.0) * daylight(sun_elevation_deg)
}

/// The colour grading of `params`; everything else (exposure, temperature, ...) stays at Bevy's
/// defaults so switching looks never leaves residue.
#[must_use]
pub fn grading_for(params: LookParams) -> ColorGrading {
    let mut g = ColorGrading::default();
    g.global.post_saturation = params.post_saturation;
    for section in [&mut g.shadows, &mut g.midtones, &mut g.highlights] {
        section.contrast = params.contrast;
    }
    g
}

/// CPU replica of Bevy's `SomewhatBoringDisplayTransform` (`bevy_core_pipeline` tonemapping.wesl).
pub(crate) fn somewhat_boring(c: [f32; 3]) -> [f32; 3] {
    let curve = |v: f32| 1.0 - (-v).exp();
    let lum = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let cb = -0.1146 * c[0] - 0.3854 * c[1] + 0.5 * c[2];
    let cr = 0.5 * c[0] - 0.4542 * c[1] - 0.0458 * c[2];
    let bt = curve(cb.hypot(cr) * 2.4);
    let desat = ((bt - 0.7) * 0.8).max(0.0).powi(2);
    let k = (curve(lum) / lum.max(1e-5)).max(0.0);
    let mut out = [0.0; 3];
    for (i, o) in out.iter_mut().enumerate() {
        let tm0 = c[i] * k;
        let tm1 = curve(c[i] + (lum - c[i]) * desat);
        *o = (tm0 + (tm1 - tm0) * bt * bt) * 0.97;
    }
    out
}

/// The scene-linear colour a camera with `Hdr` must clear to so that, after its tonemapper, the
/// sky region shows `display` (linear RGB `0..=1`), exactly what the 8-bit camera showed when the
/// clear colour bypassed the tonemapper (it was written straight to the target).
///
/// `SomewhatBoringDisplayTransform` is inverted numerically with its CPU replica, `None` is the
/// identity, every other tonemapper is approximated by an inverse Reinhard curve `x / (1 + x)` (an
/// approximation: the LUT based ones cannot be evaluated on the CPU). Colour grading (the
/// `Vivid` post saturation of 1.1) is not inverted.
#[must_use]
pub fn hdr_clear_color(tonemapping: Tonemapping, display: [f32; 3]) -> [f32; 3] {
    // The curves reach 1 only at infinity; stay below the white the target can show.
    let target = display.map(|v| v.clamp(0.0, 0.97));
    match tonemapping {
        Tonemapping::None => display,
        Tonemapping::SomewhatBoringDisplayTransform => {
            let target = display.map(|v| v.clamp(0.0, 0.9));
            let mut x = target.map(|v| v.max(1e-4));
            for _ in 0..48 {
                let y = somewhat_boring(x);
                for i in 0..3 {
                    x[i] = (x[i] * target[i] / y[i].max(1e-5)).clamp(1e-4, 64.0);
                }
            }
            x
        }
        _ => target.map(|v| v / (1.0 - v)),
    }
}
