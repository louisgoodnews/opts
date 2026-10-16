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
    }
}
