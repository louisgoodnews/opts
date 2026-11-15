//! The printed summary block of the measurement.

use crate::{Totals, ENCODERS};

pub(crate) fn report(t: &Totals) {
    let mp = t.base_pixels as f64 / 1e6;
    println!(
        "sampled textures: {} ({:.1} Mpx base, {:.1} Mpx in levels 1..)",
        t.textures,
        mp,
        t.chain_pixels as f64 / 1e6
    );
    println!(
        "DXT1 textures with transparent texels: {}",
        t.bc1_with_alpha
    );
    let per = |s: f64| s * 1000.0 / t.textures.max(1) as f64;
    println!(
        "option 1 decode:  {:.2} ms/texture ({:.0} Mpx/s)",
        per(t.decode_s),
        mp / t.decode_s.max(1e-9)
    );
    println!("option 1 chain:   {:.2} ms/texture", per(t.chain_s));
    for (e, name) in ENCODERS.iter().enumerate() {
        let psnr = |sq: f64, n: f64| {
            if sq == 0.0 {
                f64::INFINITY
            } else {
                10.0 * (255.0f64 * 255.0 / (sq / n)).log10()
            }
        };
        println!(
            "option 2 encode, {name}: {:.2} ms/texture ({:.1} MB/s of RGBA input), PSNR rgb {:.2} dB, alpha DXT1 {:.2} dB, alpha DXT3/5 {:.2} dB",
            per(t.encode_s[e]),
            t.chain_pixels as f64 * 4.0 / 1e6 / t.encode_s[e].max(1e-9),
            psnr(t.sq_err[e], t.chain_pixels as f64 * 3.0),
            psnr(t.alpha_sq_err[0][e], t.alpha_samples[0].max(1) as f64),
            psnr(t.alpha_sq_err[1][e], t.alpha_samples[1].max(1) as f64),
        );
    }
}
