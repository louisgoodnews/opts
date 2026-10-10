//! Bundled fonts and the icon font.
//!
//! Bevy 0.14 draws text with a single font per text section and has no font fallback. Its built-in
//! default is an ASCII-only subset, so [`OptsGuiPlugin`](crate::OptsGuiPlugin) replaces it with
//! **DejaVu Sans** (Latin incl. umlauts, Greek, arrows, geometric shapes, check marks) and loads
//! **Material Icons** as a separate font ([`ICON_FONT`]) for pictograms (see [`crate::icons`]).
//!
//! Licences: DejaVu Sans — Bitstream Vera / DejaVu licence (free to bundle); Material Icons —
//! Apache 2.0 (`assets/fonts/LICENSE-MaterialIcons.txt`).

use bevy::prelude::*;

/// Raw bytes of the bundled DejaVu Sans regular font.
pub const DEJAVU_SANS: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
/// Raw bytes of the bundled Material Icons font.
pub const MATERIAL_ICONS: &[u8] = include_bytes!("../assets/fonts/MaterialIcons-Regular.ttf");

/// Fixed handle of the Material Icons font; use it as `TextStyle::font` for icon glyphs.
/// A constant handle means no system needs a resource to spawn icons.
pub const ICON_FONT: Handle<Font> = Handle::Uuid(
    bevy::asset::uuid::Uuid::from_u128(0x6f70_7473_6963_6f6e_6600_0000_0000_0001),
    std::marker::PhantomData,
);

/// Installs DejaVu Sans as the default font and registers the [`ICON_FONT`].
pub fn install_fonts(fonts: Option<ResMut<Assets<Font>>>) {
    let Some(mut fonts) = fonts else {
        tracing::debug!("no font assets (headless): skipping font installation");
        return;
    };
    let _ = fonts.insert(
        Handle::<Font>::default().id(),
        Font::from_bytes(DEJAVU_SANS.to_vec()),
    );
    let _ = fonts.insert(ICON_FONT.id(), Font::from_bytes(MATERIAL_ICONS.to_vec()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers(font: &[u8], ch: char) -> bool {
        ttf_has_glyph(font, ch)
    }

    /// Minimal cmap lookup through `ab_glyph` (the rasteriser Bevy uses).
    fn ttf_has_glyph(font: &[u8], ch: char) -> bool {
        use ab_glyph::Font as _;
        ab_glyph::FontRef::try_from_slice(font).is_ok_and(|f| f.glyph_id(ch).0 != 0)
    }

    #[test]
    fn dejavu_covers_german_text_and_ui_symbols() {
        for ch in "äöüÄÖÜß€°·•–—−μ✓✕✖ℹ↺▶▼●←↑→↓↖↗↘↙„“".chars()
        {
            assert!(
                covers(DEJAVU_SANS, ch),
                "DejaVu Sans lacks U+{:04X} {ch}",
                ch as u32
            );
        }
    }

    #[test]
    fn material_icons_cover_every_named_icon() {
        for (name, icon) in crate::icons::ALL {
            let ch = icon.chars().next().expect("icon is one char");
            assert!(covers(MATERIAL_ICONS, ch), "Material Icons lacks {name}");
        }
    }
}
