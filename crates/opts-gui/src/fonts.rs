//! Bundled fonts and the icon font.
//!
//! [`OptsGuiPlugin`](crate::OptsGuiPlugin) registers the UI family (see [`UI_FAMILY`]) with
//! several weights, replaces Bevy's default font by its regular face and loads **Material Icons**
//! as a separate font ([`ICON_FONT`]) for pictograms (see [`crate::icons`]). Text selects its
//! face by family name and weight; a CSS-style stack ([`UI_FONT_STACK`]) puts **DejaVu Sans**
//! behind the UI family so glyphs the family lacks (arrows, geometric shapes, check marks) still
//! draw.
//!
//! Licences: see `docs/src/legal/licences.md` and `assets/fonts/LICENSE-*.txt`.

use bevy::prelude::*;

/// Family name of the UI typeface as embedded in its files.
pub const UI_FAMILY: &str = "Noto Sans";
/// Family name of the fallback typeface (glyphs the UI family lacks: arrows, geometric shapes).
pub const FALLBACK_FAMILY: &str = "DejaVu Sans";
/// Family name of the monospaced companion (diagnostics, logs).
pub const MONO_FAMILY: &str = "JetBrains Mono";
/// The family stack of all UI text.
pub const UI_FONT_STACK: &str = "Noto Sans, DejaVu Sans";

/// Regular face of the UI family (also Bevy's default font).
pub const UI_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
/// Medium face (weight 500) of the UI family.
pub const UI_MEDIUM: &[u8] = include_bytes!("../assets/fonts/NotoSans-Medium.ttf");
/// Bold face (weight 700) of the UI family; weights 600 and above use it.
pub const UI_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");
/// Regular face of the monospaced companion.
pub const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
/// Raw bytes of the bundled DejaVu Sans regular font (the fallback behind the UI family).
pub const DEJAVU_SANS: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
/// Raw bytes of the bundled Material Icons font.
pub const MATERIAL_ICONS: &[u8] = include_bytes!("../assets/fonts/MaterialIcons-Regular.ttf");

const fn font_handle(low: u128) -> Handle<Font> {
    Handle::Uuid(
        bevy::asset::uuid::Uuid::from_u128(0x6f70_7473_6963_6f6e_6600_0000_0000_0000 | low),
        std::marker::PhantomData,
    )
}

/// Fixed handle of the Material Icons font; use it as `TextFont::font` for icon glyphs.
/// A constant handle means no system needs a resource to spawn icons.
pub const ICON_FONT: Handle<Font> = font_handle(1);
/// Fixed handle under which the Medium face is registered.
pub const MEDIUM_FONT: Handle<Font> = font_handle(2);
/// Fixed handle under which the Bold face is registered.
pub const BOLD_FONT: Handle<Font> = font_handle(3);
/// Fixed handle under which the monospaced face is registered.
pub const MONO_FONT: Handle<Font> = font_handle(4);
/// Fixed handle under which the fallback face is registered.
pub const FALLBACK_FONT: Handle<Font> = font_handle(5);

/// The body size of the type scale in logical pixels: [`UiTypography::base_px`] scales every
/// style by its ratio to this size.
pub const BASE_TEXT_PX: f32 = 14.0;
/// The smallest and largest base text size the user may choose.
pub const BASE_TEXT_RANGE: std::ops::RangeInclusive<u32> = 12..=24;

/// The user's text preferences (accessibility): the family put in front of the UI stack and the
/// base text size. Written from the settings `ui_font` and `ui_font_size`; read by
/// `sync_typestyle_tokens`, so a change reaches all text with a type token in the next frame.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct UiTypography {
    /// A font family put in front of the UI stack; empty for the bundled UI family.
    pub family: String,
    /// The size of body text in logical pixels (12 to 24); every style scales with it.
    pub base_px: f32,
}

impl Default for UiTypography {
    fn default() -> Self {
        Self {
            family: String::new(),
            base_px: BASE_TEXT_PX,
        }
    }
}

impl UiTypography {
    /// A text size of the type scale after the base size: scaled by `base_px / 14` and never
    /// below [`MIN_TEXT_PX`](crate::theme::typography::MIN_TEXT_PX).
    #[must_use]
    pub fn size(&self, style_px: f32) -> f32 {
        (style_px * self.base_px / BASE_TEXT_PX).max(crate::theme::typography::MIN_TEXT_PX)
    }

    /// The font source of ordinary UI text: the chosen family (if any) before the UI stack.
    #[must_use]
    pub fn font_source(&self) -> FontSource {
        ui_font_source_for(&self.family)
    }
}

/// The font source of all ordinary UI text: the UI family with its fallback.
pub fn ui_font_source() -> FontSource {
    FontSource::Families(UI_FONT_STACK.into())
}

/// The UI stack with `family` in front (a family name containing a comma or a quote is ignored).
pub fn ui_font_source_for(family: &str) -> FontSource {
    let family = family.trim();
    if family.is_empty() || family.contains([',', '"', '\'']) || family == UI_FAMILY {
        return ui_font_source();
    }
    FontSource::Families(format!("\"{family}\", {UI_FONT_STACK}").into())
}

/// `true` when `source` is Bevy's default font or already a UI stack, with or without a chosen
/// family in front (the sources the theme may restyle); `false` for the icon font and any other
/// explicit choice.
pub fn is_ui_font(source: &FontSource) -> bool {
    match source {
        FontSource::Handle(handle) => *handle == Handle::<Font>::default(),
        FontSource::Families(s) => s.as_str().ends_with(UI_FONT_STACK),
        _ => false,
    }
}

/// Moves all text that still uses Bevy's default font onto the UI family stack, so weights and
/// fallbacks work for text that does not carry a type token either.
pub fn apply_ui_font(mut query: Query<&mut TextFont, Changed<TextFont>>) {
    for mut font in &mut query {
        if matches!(&font.font, FontSource::Handle(h) if *h == Handle::<Font>::default()) {
            font.font = ui_font_source();
        }
    }
}

/// Installs the UI family as the default font, registers its other weights, the monospaced
/// companion, the fallback and the [`ICON_FONT`].
pub fn install_fonts(fonts: Option<ResMut<Assets<Font>>>) {
    let Some(mut fonts) = fonts else {
        tracing::debug!("no font assets (headless): skipping font installation");
        return;
    };
    let _ = fonts.insert(
        Handle::<Font>::default().id(),
        Font::from_bytes(UI_REGULAR.to_vec()),
    );
    for (handle, bytes) in [
        (&MEDIUM_FONT, UI_MEDIUM),
        (&BOLD_FONT, UI_BOLD),
        (&MONO_FONT, MONO_REGULAR),
        (&FALLBACK_FONT, DEJAVU_SANS),
        (&ICON_FONT, MATERIAL_ICONS),
    ] {
        let _ = fonts.insert(handle.id(), Font::from_bytes(bytes.to_vec()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers(font: &[u8], ch: char) -> bool {
        use ab_glyph::Font as _;
        ab_glyph::FontRef::try_from_slice(font).is_ok_and(|f| f.glyph_id(ch).0 != 0)
    }

    #[test]
    fn a_chosen_family_goes_in_front_of_the_stack_and_odd_names_are_ignored() {
        assert_eq!(ui_font_source_for(""), ui_font_source());
        assert_eq!(ui_font_source_for("Noto Sans"), ui_font_source());
        assert_eq!(ui_font_source_for("a, b"), ui_font_source());
        let f = ui_font_source_for("Atkinson Hyperlegible");
        assert_eq!(
            f,
            FontSource::Families("\"Atkinson Hyperlegible\", Noto Sans, DejaVu Sans".into())
        );
        assert!(is_ui_font(&f) && is_ui_font(&ui_font_source()));
        assert!(!is_ui_font(&FontSource::Family(
            "Atkinson Hyperlegible".into()
        )));
    }

    #[test]
    fn the_base_size_scales_the_scale_and_keeps_the_floor() {
        let t = UiTypography {
            family: String::new(),
            base_px: 20.0,
        };
        assert!((t.size(14.0) - 20.0).abs() < 1e-4);
        assert!((t.size(40.0) - 40.0 * 20.0 / 14.0).abs() < 1e-3);
        let small = UiTypography {
            family: String::new(),
            base_px: 12.0,
        };
        assert_eq!(small.size(12.0), 12.0, "the floor holds");
        assert_eq!(
            UiTypography::default().size(16.0),
            16.0,
            "14 px is the identity"
        );
    }

    #[test]
    fn the_stack_covers_german_text_and_ui_symbols() {
        // Each character must come from the UI family or, failing that, the fallback.
        for ch in "äöüÄÖÜß€°·•–—−μ✓✕✖ℹ↺▶▼●←↑→↓↖↗↘↙„“".chars()
        {
            assert!(
                covers(UI_REGULAR, ch) || covers(DEJAVU_SANS, ch),
                "neither Noto Sans nor DejaVu Sans has U+{:04X} {ch}",
                ch as u32
            );
        }
        // The text of the product itself needs no fallback.
        for ch in "äöüÄÖÜß€°·–—„“".chars() {
            assert!(covers(UI_REGULAR, ch), "Noto Sans lacks {ch}");
        }
    }

    #[test]
    fn all_weights_are_distinct_faces_of_one_family() {
        assert_ne!(UI_REGULAR.len(), UI_MEDIUM.len());
        assert_ne!(UI_MEDIUM.len(), UI_BOLD.len());
        for bytes in [UI_REGULAR, UI_MEDIUM, UI_BOLD, MONO_REGULAR, DEJAVU_SANS] {
            assert!(ab_glyph::FontRef::try_from_slice(bytes).is_ok());
        }
    }

    #[test]
    fn material_icons_cover_every_named_icon() {
        for (name, icon) in crate::icons::ALL {
            let ch = icon.chars().next().expect("icon is one char");
            assert!(covers(MATERIAL_ICONS, ch), "Material Icons lacks {name}");
        }
    }

    #[test]
    fn the_default_and_the_stack_are_recognised_but_the_icon_font_is_not() {
        assert!(is_ui_font(&FontSource::default()));
        assert!(is_ui_font(&ui_font_source()));
        assert!(!is_ui_font(&FontSource::Handle(ICON_FONT)));
        assert!(!is_ui_font(&FontSource::Family("Anything".into())));
    }
}
