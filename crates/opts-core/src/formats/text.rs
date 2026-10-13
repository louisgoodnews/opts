//! Shared text decoding and block parsing utilities for OMSI configuration formats.
//!
//! OMSI text configuration files (`.bus`, `paths.cfg`, `Money/*.cfg`, `.sco`, etc.)
//! use a common keyword-driven structure where keywords appear in square brackets
//! (`[keyword]`) followed by parameter lines.

use glam::Vec3;

/// Decodes bytes to a UTF-8 [`String`].
///
/// If the input is valid UTF-8, it is decoded directly. If invalid UTF-8 bytes
/// are encountered, it falls back to Windows-1252 (CP1252) decoding, which is
/// widely used in legacy German OMSI add-ons.
#[must_use]
pub fn decode_bytes(bytes: &[u8]) -> String {
    // Check for UTF-16LE BOM (used by most OMSI .map, global.cfg, laststn.osn files)
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let u16_slice: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&u16_slice);
    }

    // Check for UTF-16BE BOM
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let u16_slice: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&u16_slice);
    }

    // Check for and strip UTF-8 BOM if present
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };

    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    // Windows-1252 fallback
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        match b {
            0x00..=0x7F => out.push(b as char),
            0x80 => out.push('€'),
            0x81 => out.push('\u{0081}'),
            0x82 => out.push('‚'),
            0x83 => out.push('ƒ'),
            0x84 => out.push('„'),
            0x85 => out.push('…'),
            0x86 => out.push('†'),
            0x87 => out.push('‡'),
            0x88 => out.push('ˆ'),
            0x89 => out.push('‰'),
            0x8A => out.push('Š'),
            0x8B => out.push('‹'),
            0x8C => out.push('Œ'),
            0x8D => out.push('\u{008D}'),
            0x8E => out.push('Ž'),
            0x8F => out.push('\u{008F}'),
            0x90 => out.push('\u{0090}'),
            0x91 => out.push('‘'),
            0x92 => out.push('’'),
            0x93 => out.push('“'),
            0x94 => out.push('”'),
            0x95 => out.push('•'),
            0x96 => out.push('–'),
            0x97 => out.push('—'),
            0x98 => out.push('˜'),
            0x99 => out.push('™'),
            0x9A => out.push('š'),
            0x9B => out.push('›'),
            0x9C => out.push('œ'),
            0x9D => out.push('\u{009D}'),
            0x9E => out.push('ž'),
            0x9F => out.push('Ÿ'),
            0xA0..=0xFF => out.push(b as char),
        }
    }
    out
}

/// Checks if a trimmed line is an OMSI keyword enclosed in `[...]`.
///
/// Returns the inner keyword name without brackets if valid.
#[must_use]
pub fn parse_keyword(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
        Some(&line[1..line.len() - 1])
    } else {
        None
    }
}

/// Represents an extracted OMSI configuration block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmsiBlock {
    /// The keyword string without square brackets (e.g. `"currency"`, `"coin"`).
    pub keyword: String,
    /// 1-based line number where the keyword occurred in the source text.
    pub line_number: usize,
    /// Parameter lines consumed by this keyword.
    pub parameters: Vec<String>,
}

/// Scans OMSI text into structured [`OmsiBlock`] instances.
///
/// Lines preceding the first keyword and lines beginning with comment
/// indicators outside parameter blocks are ignored. Every non-blank line after a keyword up to the
/// next keyword is one of its parameters; see [`scan_blocks_with`] for keywords whose parameter
/// count is known.
#[must_use]
pub fn scan_blocks(text: &str) -> Vec<OmsiBlock> {
    scan_blocks_with(text, |_| None)
}

/// [`scan_blocks`] for formats in which some keywords take a fixed number of parameter *lines*.
///
/// `arity(keyword)` returns that number (lower-case keyword). For such a keyword exactly that many
/// lines after it belong to it, **a blank line counting as an empty parameter**; further lines
/// up to the next keyword are free text between blocks (author notes without a comment marker) and are
/// dropped instead of becoming parameters. A keyword with `Some(0)` takes none. Comment lines (`//`,
/// `#`) are never parameters. Keywords with `None` behave as in [`scan_blocks`].
#[must_use]
pub fn scan_blocks_with(text: &str, arity: impl Fn(&str) -> Option<usize>) -> Vec<OmsiBlock> {
    let mut blocks = Vec::new();
    let mut current_block: Option<OmsiBlock> = None;
    // Parameter lines the current keyword still takes literally (`None`: free-form).
    let mut literal: Option<usize> = None;

    for (line_idx, raw_line) in text.lines().enumerate() {
        let line_number = line_idx + 1;
        let line = raw_line.trim_end_matches(['\r', '\n']).trim();

        // Check for keyword beginning line
        if !line.is_empty() && raw_line.starts_with('[') && raw_line.trim_end().ends_with(']') {
            if let Some(kw) = parse_keyword(raw_line.trim_end()) {
                if let Some(prev) = current_block.take() {
                    blocks.push(prev);
                }
                let keyword = kw.to_ascii_lowercase();
                literal = arity(&keyword);
                current_block = Some(OmsiBlock {
                    keyword,
                    line_number,
                    parameters: Vec::new(),
                });
                continue;
            }
        }

        let Some(block) = current_block.as_mut() else {
            continue;
        };
        if line.is_empty() {
            // A blank line is an empty parameter only where the keyword takes lines literally.
            if let Some(left) = literal.as_mut().filter(|l| **l > 0) {
                block.parameters.push(String::new());
                *left -= 1;
            }
            continue;
        }
        // Lines starting with obvious comment markers are never parameters.
        if line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        match literal.as_mut() {
            Some(0) => {} // free text between blocks
            Some(left) => {
                block.parameters.push(line.to_string());
                *left -= 1;
            }
            None => block.parameters.push(line.to_string()),
        }
    }

    if let Some(last) = current_block {
        blocks.push(last);
    }

    blocks
}

/// Helper to parse a floating point number tolerating commas as decimal separators.
pub fn parse_float(s: &str) -> Result<f32, std::num::ParseFloatError> {
    let normalized = s.trim().replace(',', ".");
    normalized.parse::<f32>()
}

/// Helper to parse a 64-bit floating point number tolerating commas as decimal separators.
pub fn parse_f64(s: &str) -> Result<f64, std::num::ParseFloatError> {
    let normalized = s.trim().replace(',', ".");
    normalized.parse::<f64>()
}

/// Helper to parse a 3D coordinate vector (`Vec3`) from three consecutive lines.
pub fn parse_vec3(slice: &[String]) -> Result<Vec3, String> {
    if slice.len() < 3 {
        return Err("Expected 3 lines for Vec3".to_string());
    }
    let x = parse_float(&slice[0]).map_err(|e| format!("Invalid X coord: {e}"))?;
    let y = parse_float(&slice[1]).map_err(|e| format!("Invalid Y coord: {e}"))?;
    let z = parse_float(&slice[2]).map_err(|e| format!("Invalid Z coord: {e}"))?;
    Ok(Vec3::new(x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_parameter_line_is_an_empty_value_not_the_next_text() {
        let text =
            "[mesh]\na.o3d\n\n[matl_transmap]\n\n\n########\n\nRauchsystem\n\n[smoke]\n-1.16\n";
        let arity = |k: &str| match k {
            "matl_transmap" => Some(1),
            "isshadow" => Some(0),
            _ => None,
        };
        let blocks = scan_blocks_with(text, arity);
        let t = blocks
            .iter()
            .find(|b| b.keyword == "matl_transmap")
            .unwrap();
        assert_eq!(
            t.parameters,
            vec![String::new()],
            "empty value, not 'Rauchsystem'"
        );
        // The old reading took the prose as the value.
        let old = scan_blocks(text);
        assert_eq!(old[1].parameters, vec!["Rauchsystem".to_string()]);
        // Following keywords are untouched.
        assert_eq!(blocks.last().unwrap().parameters, vec!["-1.16".to_string()]);
    }

    #[test]
    fn zero_parameter_keywords_take_no_following_text() {
        let text = "[isshadow]\nnote about the mesh\n[mesh]\nb.o3d\n";
        let blocks = scan_blocks_with(text, |k| (k == "isshadow").then_some(0));
        assert!(blocks[0].parameters.is_empty());
        assert_eq!(blocks[1].parameters, vec!["b.o3d".to_string()]);
    }

    #[test]
    fn a_texture_name_after_transmap_is_still_read() {
        let blocks = scan_blocks_with("[matl_transmap]\nmask.bmp\nstray\n", |k| {
            (k == "matl_transmap").then_some(1)
        });
        assert_eq!(blocks[0].parameters, vec!["mask.bmp".to_string()]);
    }

    #[test]
    fn test_decode_utf8_and_cp1252() {
        let utf8_bytes = "Hallo Welt!".as_bytes();
        assert_eq!(decode_bytes(utf8_bytes), "Hallo Welt!");

        // 0xE4 is ä in CP1252 / ISO-8859-1
        let cp1252_bytes = vec![0x47, 0x65, 0x73, 0x63, 0x68, 0x66, 0x74, 0x65, 0x20, 0xE4];
        let decoded = decode_bytes(&cp1252_bytes);
        assert!(decoded.contains('ä'));

        // 0x80 is € in Windows-1252
        let euro_bytes = vec![0x80];
        assert_eq!(decode_bytes(&euro_bytes), "€");

        // UTF-16LE with BOM
        let utf16le = vec![
            0xFF, 0xFE, b'H', 0x00, b'a', 0x00, b'l', 0x00, b'l', 0x00, b'o', 0x00,
        ];
        assert_eq!(decode_bytes(&utf16le), "Hallo");
    }

    #[test]
    fn test_parse_keyword() {
        assert_eq!(parse_keyword("[currency]"), Some("currency"));
        assert_eq!(parse_keyword("[coin]\r\n"), Some("coin"));
        assert_eq!(parse_keyword("not_a_kw"), None);
        assert_eq!(parse_keyword(" [indented]"), None);
    }

    #[test]
    fn test_scan_blocks() {
        let text = r#"
// Top comment banner
[currency]
Euro
2

# Comment between blocks
[coin]
coin_1.o3d
1.00

[coin]
coin_2.o3d
2.00
"#;
        let blocks = scan_blocks(text);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].keyword, "currency");
        assert_eq!(blocks[0].parameters, vec!["Euro", "2"]);
        assert_eq!(blocks[1].keyword, "coin");
        assert_eq!(blocks[1].parameters, vec!["coin_1.o3d", "1.00"]);
        assert_eq!(blocks[2].keyword, "coin");
        assert_eq!(blocks[2].parameters, vec!["coin_2.o3d", "2.00"]);
    }

    #[test]
    fn test_parse_float_comma() {
        assert_eq!(parse_float("1.5").unwrap(), 1.5);
        assert_eq!(parse_float("2,75").unwrap(), 2.75);
        assert_eq!(parse_f64("0,10").unwrap(), 0.10);
    }
}
