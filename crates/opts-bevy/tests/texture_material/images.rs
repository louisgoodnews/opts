//! BMP and PNG content is sniffed by the unified loader.

use super::*;

#[test]
fn test_bmp_content_sniffed_via_unified_loader() {
    // Generate a minimal 2x2 BMP image using the image crate
    let img_buf = image::RgbaImage::new(2, 2);
    let mut bmp_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bmp_bytes);
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Bmp)
        .expect("write to BMP");

    // Unified loader must sniff 'BM' and decode correctly into Bevy Image
    let img = load_bevy_image(&bmp_bytes).expect("load BMP image");
    assert_eq!(img.texture_descriptor.size.width, 2);
    assert_eq!(img.texture_descriptor.size.height, 2);
    assert_eq!(img.texture_descriptor.format, TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn test_png_content_sniffed_via_unified_loader() {
    let img_buf = image::RgbaImage::new(3, 3);
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    img_buf
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("write to PNG");

    let img = load_bevy_image(&png_bytes).expect("load PNG image");
    assert_eq!(img.texture_descriptor.size.width, 3);
    assert_eq!(img.texture_descriptor.size.height, 3);
}
