//! Text rendering: colours, the alpha bitmap, fitting long text, and the text sources.

use super::*;
use super::fixtures::*;

#[test]
fn mono_uses_text_colour_and_colour_mode_uses_bitmap() {
    let (font, bmp) = demo();
    let mono = render_rgba(&font, &bmp, &spec("A", 4, 4, 0));
    assert_eq!(&mono[0..4], &[200, 0, 0, 255]);
    let mut s = spec("A", 4, 4, 0);
    s.mono = false;
    let col = render_rgba(&font, &bmp, &s);
    assert_eq!(&col[0..4], &[10, 20, 30, 255]);
}

#[test]
fn alpha_bitmap_cuts_the_glyph() {
    let (font, bmp) = demo();
    let buf = render_rgba(&font, &bmp, &spec("B", 4, 4, 0));
    assert_eq!(alpha_row(&buf, 4, 0), [255, 255, 0, 0]);
}

#[test]
fn long_text_shrinks_to_fit_and_empty_text_is_transparent() {
    let (font, bmp) = demo();
    let buf = render_rgba(&font, &bmp, &spec("AB", 4, 4, 1));
    // Scale 0.5: the 4-px-high line occupies the middle two rows.
    assert_eq!(alpha_row(&buf, 4, 0), [0, 0, 0, 0]);
    assert!(alpha_row(&buf, 4, 1).iter().any(|a| *a > 0));
    assert!(render_rgba(&font, &bmp, &spec("", 4, 4, 0))
        .iter()
        .all(|b| *b == 0));
}

#[test]
fn text_source_by_number_and_by_name() {
    let vars = vec!["Alexanderplatz".to_string(), "5".to_string()];
    let names = vec!["Name".to_string(), "Kind".to_string()];
    assert_eq!(
        resolve_text(0, None, &vars, &names).as_deref(),
        Some("Alexanderplatz")
    );
    assert_eq!(
        resolve_text(0, Some("kind"), &vars, &names).as_deref(),
        Some("5")
    );
    assert_eq!(resolve_text(0, Some("missing"), &vars, &names), None);
    assert_eq!(resolve_text(3, None, &vars, &names), None);
}
