//! Bitmap font definitions (`Fonts/*.oft`).
//!
//! A file holds one or more `[newfont]` blocks (name, colour bitmap, alpha bitmap, line height,
//! horizontal gap) each followed by `[char]` entries (character, x start, x end including the gap,
//! y top). See `docs/src/formats/text/fonts.md`.

use std::collections::HashMap;

use super::text::decode_bytes;

/// One glyph cell in the font bitmaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OftChar {
    /// First pixel column of the glyph.
    pub x_start: u32,
    /// Column after the glyph, including the spacing to the next one.
    pub x_end: u32,
    /// Top pixel row of the glyph cell.
    pub y_top: u32,
}

/// A bitmap font.
#[derive(Debug, Clone, PartialEq)]
pub struct OftFont {
    /// Registry name used by `[texttexture]` (compared case-insensitively).
    pub name: String,
    /// Colour bitmap file name (relative to the `.oft`).
    pub bitmap: String,
    /// Alpha bitmap file name.
    pub alpha: String,
    /// Glyph cell height in pixels, including vertical spacing.
    pub line_height: u32,
    /// Extra horizontal pixels between glyphs.
    pub gap: u32,
    /// Glyph cells by character.
    pub chars: HashMap<char, OftChar>,
}

/// Parsed `.oft` file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OftFile {
    /// Fonts in file order.
    pub fonts: Vec<OftFont>,
}

impl OftFile {
    /// Parses a font file. Malformed `[char]` entries are skipped; a `[newfont]` without its five
    /// lines is an error.
    ///
    /// # Errors
    /// Returns a message when a `[newfont]` block is incomplete or has a non-numeric height/gap.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let text = decode_bytes(bytes);
        let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        let mut fonts: Vec<OftFont> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let key = lines[i].trim_end().to_ascii_lowercase();
            match key.as_str() {
                "[newfont]" => {
                    let p = lines
                        .get(i + 1..i + 6)
                        .ok_or_else(|| format!("line {}: [newfont] needs 5 lines", i + 1))?;
                    let num = |s: &str, what: &str| {
                        s.trim()
                            .parse::<u32>()
                            .map_err(|_| format!("line {}: bad {what} `{s}`", i + 1))
                    };
                    fonts.push(OftFont {
                        name: p[0].trim().to_string(),
                        bitmap: p[1].trim().to_string(),
                        alpha: p[2].trim().to_string(),
                        line_height: num(p[3], "line height")?,
                        gap: num(p[4], "gap")?,
                        chars: HashMap::new(),
                    });
                    i += 6;
                }
                "[char]" => {
                    if let (Some(font), Some(p)) = (fonts.last_mut(), lines.get(i + 1..i + 5)) {
                        let ch = p[0].chars().next();
                        let n = |s: &str| s.trim().parse::<u32>().ok();
                        if let (Some(ch), Some(x0), Some(x1), Some(y)) =
                            (ch, n(p[1]), n(p[2]), n(p[3]))
                        {
                            font.chars.insert(
                                ch,
                                OftChar {
                                    x_start: x0,
                                    x_end: x1,
                                    y_top: y,
                                },
                            );
                        }
                    }
                    i += 5;
                }
                _ => i += 1,
            }
        }
        Ok(Self { fonts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fonts_and_chars() {
        let src = "\tcomment\r\n[newfont]\r\nDemo\r\nd.bmp\r\nd_a.bmp\r\n16\r\n2\r\n[char]\r\nA\r\n0\r\n9\r\n0\r\n[char]\r\n \r\n9\r\n13\r\n0\r\n[char]\r\nB\r\nx\r\n1\r\n2\r\n";
        let f = OftFile::parse(src.as_bytes()).expect("parse");
        assert_eq!(f.fonts.len(), 1);
        let font = &f.fonts[0];
        assert_eq!(
            (font.name.as_str(), font.line_height, font.gap),
            ("Demo", 16, 2)
        );
        assert_eq!(font.chars[&'A'].x_end, 9);
        assert_eq!(font.chars[&' '].x_start, 9);
        assert!(!font.chars.contains_key(&'B'));
    }

    #[test]
    fn incomplete_font_block_is_an_error() {
        assert!(OftFile::parse(b"[newfont]\nDemo\n").is_err());
    }
}
