//! Decoding of text files: UTF-8 with a Windows-1252 fallback.

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
