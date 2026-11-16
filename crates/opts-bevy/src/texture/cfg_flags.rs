//! Texture property flags from `<texture>.cfg` (see `docs/src/formats/text/texture-cfg.md`):
//! `[moisture]` (the texture gets wet in rain), `[puddles]` (puddles may form on it) and the
//! `[surface]` number. Only unindented keyword lines count; indented lines are the tutorial text
//! the authors pasted into the files.

/// The flags one texture property file sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextureFlags {
    /// `[moisture]` present: the texture takes the global wetness.
    pub moisture: bool,
    /// `[puddles]` present: puddles may form on the texture (❓ on non-ground objects).
    pub puddles: bool,
    /// The number on the line after `[surface]`, when it parses.
    pub surface: Option<u32>,
}

impl TextureFlags {
    /// Reads the flags from the text of a property file. Keywords are case-insensitive; a file
    /// without them gives the default (dry, no puddles, no surface).
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut flags = Self::default();
        let mut lines = text.lines();
        while let Some(line) = lines.next() {
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            match line.trim().to_ascii_lowercase().as_str() {
                "[moisture]" => flags.moisture = true,
                "[puddles]" => flags.puddles = true,
                "[surface]" => {
                    flags.surface = lines
                        .by_ref()
                        .map(str::trim)
                        .find(|v| !v.is_empty())
                        .and_then(|v| v.parse().ok());
                }
                _ => {}
            }
        }
        flags
    }
}

/// The wetness a texture takes at the global wetness `wetness` (0..=1): the full value when it
/// has `[moisture]`, dry (0) otherwise. The one rule for objects and vehicles.
#[must_use]
pub fn texture_wetness(flags: TextureFlags, wetness: f32) -> f32 {
    if flags.moisture {
        wetness.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WET: &str = "[moisture]\n[puddles]\n[surface]\n6\n";
    const TUTORIAL: &str = "[surface]\n0\n\t[moisture] pasted tutorial\n\t[surface]\n\t15\n";

    #[test]
    fn keywords_and_surface_are_read() {
        let f = TextureFlags::parse(WET);
        assert!(f.moisture && f.puddles);
        assert_eq!(f.surface, Some(6));
    }

    #[test]
    fn indented_tutorial_lines_are_ignored() {
        let f = TextureFlags::parse(TUTORIAL);
        assert!(!f.moisture && !f.puddles);
        assert_eq!(f.surface, Some(0));
    }

    #[test]
    fn empty_and_crlf_files_parse() {
        assert_eq!(TextureFlags::parse(""), TextureFlags::default());
        let f = TextureFlags::parse("[MOISTURE]\r\n[surface]\r\n 8 \r\n");
        assert!(f.moisture);
        assert_eq!(f.surface, Some(8));
    }

    #[test]
    fn corpus_style_counts_over_a_synthetic_fixture() {
        let fixture = [
            "[moisture]\n[puddles]\n[surface]\n6\n",
            "[moisture]\n",
            "[puddles]\n[surface]\n15\n",
            "[surface]\n1\n",
        ];
        let parsed: Vec<_> = fixture.iter().map(|t| TextureFlags::parse(t)).collect();
        assert_eq!(parsed.iter().filter(|f| f.moisture).count(), 2);
        assert_eq!(parsed.iter().filter(|f| f.puddles).count(), 2);
        assert_eq!(parsed.iter().filter(|f| f.surface.is_some()).count(), 3);
    }

    #[test]
    fn wetness_applies_only_with_the_moisture_flag() {
        let wet = TextureFlags::parse(WET);
        let dry = TextureFlags::parse("[puddles]\n");
        assert_eq!(texture_wetness(wet, 1.0), 1.0);
        assert_eq!(texture_wetness(wet, 0.4), 0.4);
        assert_eq!(texture_wetness(wet, 3.0), 1.0);
        assert_eq!(texture_wetness(dry, 1.0), 0.0);
    }
}
