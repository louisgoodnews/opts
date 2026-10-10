//! Renders a `[texttexture]` string with an OMSI bitmap font into a PNG.
//!
//! `cargo run --release -p opts-bevy --example render_text_texture -- <OMSI dir> <font> <text> <w> <h> <out.png> [orientation] [mono 0/1] [r g b]`
use bevy::asset::Assets;
use bevy::image::Image;
use opts_bevy::install::OmsiInstall;
use opts_bevy::text_texture::{FontRegistry, TextSpec};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 7 {
        eprintln!("usage: render_text_texture <OMSI dir> <font> <text> <w> <h> <out.png> [orientation] [mono] [r g b]");
        std::process::exit(2);
    }
    let install = OmsiInstall::new(&a[1]);
    let get = |i: usize, d: u32| a.get(i).and_then(|v| v.parse().ok()).unwrap_or(d);
    let spec = TextSpec {
        font: a[2].clone(),
        text: a[3].clone(),
        width: get(4, 256),
        height: get(5, 32),
        mono: get(8, 0) == 0,
        rgb: [get(9, 0) as u8, get(10, 0) as u8, get(11, 0) as u8],
        orientation: get(7, 0),
    };
    let mut images = Assets::<Image>::default();
    let mut fonts = FontRegistry::default();
    let Some(h) = fonts.render(&install, &mut images, &spec) else {
        eprintln!("font `{}` could not be rendered", spec.font);
        std::process::exit(1);
    };
    let img = images.get(&h).expect("image");
    // Composite onto a light grey backdrop so transparent text is visible in viewers.
    let flat: Vec<u8> = img
        .data
        .chunks_exact(4)
        .flat_map(|p| {
            let k = u32::from(p[3]);
            [0, 1, 2].map(|c| ((u32::from(p[c]) * k + 200 * (255 - k)) / 255) as u8)
        })
        .collect();
    image::save_buffer(
        &a[6],
        &flat,
        spec.width,
        spec.height,
        image::ColorType::Rgb8,
    )
    .expect("save png");
    println!("wrote {}", a[6]);
}
