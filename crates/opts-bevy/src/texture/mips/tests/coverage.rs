//! Coverage mode: the share of a level's texels above the alpha cutoff is kept.

use super::*;

#[test]
fn coverage_mode_keeps_the_share_above_the_cutoff() {
    // A thin vertical line (1 of 8 columns opaque): the plain average of level 2 is 0.25 alpha
    // at best, i.e. below the cutoff, so the line would vanish.
    let line = |x: u32, _| {
        if x == 3 {
            [0, 200, 0, 255]
        } else {
            [0, 100, 0, 0]
        }
    };
    let share = |data: &[u8], w: u32, h: u32, level: u32| {
        let off = level_offset(8, 8, level);
        let (lw, lh) = level_size(w, h, level);
        let px = &data[off..off + (lw * lh * 4) as usize];
        px.chunks_exact(4).filter(|p| p[3] >= 128).count() as f32 / (lw * lh) as f32
    };
    let mut plain = image(8, 8, line);
    generate_mip_chain(&mut plain, MipOptions::default());
    let mut cov = image(8, 8, line);
    generate_mip_chain(
        &mut cov,
        MipOptions {
            alpha_cutoff: Some(0.5),
            ..MipOptions::default()
        },
    );
    let (pd, cd) = (
        plain.data.as_ref().expect("d"),
        cov.data.as_ref().expect("d"),
    );
    assert!((share(cd, 8, 8, 0) - 0.125).abs() < 1e-6);
    assert_eq!(share(pd, 8, 8, 2), 0.0, "plain averaging loses the line");
    // Texel quantisation: a 2x2 level can only reach 0.25 steps, so it overshoots.
    let kept = share(cd, 8, 8, 2);
    assert!((0.125..=0.5).contains(&kept), "coverage kept: {kept}");
}
