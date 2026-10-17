//! Loads a base texture and a `[matl_transmap]` texture as the game does and reports what the
//! alpha composition sees: formats, sizes, and the alpha the composed texel would get.
//! Usage: `transmap_probe <base> <transmap>`
use opts_bevy::texture::{decode_to_rgba8, load_bevy_image};

fn main() {
    let mut a = std::env::args().skip(1);
    let (bp, tp) = (a.next().expect("base"), a.next().expect("transmap"));
    for (label, p) in [("base", &bp), ("trans", &tp)] {
        let mut img = load_bevy_image(&std::fs::read(p).expect("read")).expect("decode");
        println!(
            "{label}: loaded {:?} {}x{} mips {}",
            img.texture_descriptor.format,
            img.width(),
            img.height(),
            img.texture_descriptor.mip_level_count
        );
        opts_bevy::texture_filter::prepare_object_texture(&mut img);
        println!(
            "{label}: prepared {:?} mips {} bytes {}",
            img.texture_descriptor.format,
            img.texture_descriptor.mip_level_count,
            img.data.as_ref().map_or(0, Vec::len)
        );
        match decode_to_rgba8(&img) {
            Some(d) => {
                let data = d.data.as_deref().unwrap_or(&[]);
                let n = (d.width() * d.height()) as usize;
                let opaque = data.chunks_exact(4).take(n).filter(|p| p[3] > 128).count();
                println!(
                    "{label}: decoded {:?} {}x{}, alpha > 128 on {:.1} % of the texels",
                    d.texture_descriptor.format,
                    d.width(),
                    d.height(),
                    100.0 * opaque as f64 / n.max(1) as f64
                );
            }
            None => println!("{label}: decode_to_rgba8 returned None"),
        }
        if label == "trans" {
            // The value `compose_alpha` takes: an R8 map's value, else the alpha channel if the
            // map has one, else red.
            let n = (img.width() * img.height()) as usize;
            let values: Option<Vec<u8>> = if img.texture_descriptor.format
                == bevy::render::render_resource::TextureFormat::R8Unorm
            {
                img.data
                    .as_deref()
                    .map(|d| d.iter().take(n).copied().collect())
            } else {
                decode_to_rgba8(&img).and_then(|d| {
                    let data = d.data.as_deref()?;
                    let px: Vec<&[u8]> = data.chunks_exact(4).take(n).collect();
                    let has_alpha = px.iter().any(|p| p[3] != 255);
                    Some(
                        px.iter()
                            .map(|p| if has_alpha { p[3] } else { p[0] })
                            .collect(),
                    )
                })
            };
            if let Some(values) = values {
                let mut hist = [0usize; 5];
                for &v in &values {
                    hist[match v {
                        0..=50 => 0,
                        51..=152 => 1,
                        153..=203 => 2,
                        204..=242 => 3,
                        _ => 4,
                    }] += 1;
                }
                let pct = |c: usize| 100.0 * c as f64 / n.max(1) as f64;
                println!(
                    "trans: composed alpha <0.2 {:.1} %, 0.2-0.6 {:.1} %, 0.6-0.8 {:.1} %, 0.8-0.95 {:.1} %, >=0.95 {:.1} % (body/glass split from {:.0} %)",
                    pct(hist[0]),
                    pct(hist[1]),
                    pct(hist[2]),
                    pct(hist[3]),
                    pct(hist[4]),
                    100.0 * opts_bevy::model::TRANSMAP_SPLIT_SHARE
                );
            }
        }
    }
}
