//! 16-byte header validator for Microsoft DirectX `.x` files.
//!
//! Format specification per `docs/src/formats/binary/x.md`:
//! - Bytes 0..4: Magic `xof ` (ASCII)
//! - Bytes 4..8: Version `0303` or `0302` (ASCII)
//! - Bytes 8..12: Format `txt ` or `bin ` (ASCII)
//! - Bytes 12..16: Float size `0032` or `0064` (ASCII)

use crate::formats::x::error::XParseError;

/// Mandatory magic prefix for DirectX `.x` files.
pub const XOF_MAGIC: &[u8; 4] = b"xof ";

/// Length of the standard DirectX `.x` header in bytes.
pub const HEADER_LEN: usize = 16;

/// Parsed metadata from the 16-byte DirectX `.x` header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XHeader {
    /// Major and minor version numbers (e.g. `(3, 3)` for `0303`).
    pub version: (u8, u8),
    /// Bit width of floating-point values (32 or 64).
    pub float_bits: u8,
}

/// Parses and validates the 16-byte DirectX `.x` header.
///
/// # Arguments
/// * `bytes` - Raw byte slice of the file.
///
/// # Returns
/// * `Ok((XHeader, &[u8]))` - Header metadata and the remaining byte slice.
/// * `Err(XParseError)` - If the buffer is truncated, has invalid magic, or is an unsupported variant.
pub fn parse_header(bytes: &[u8]) -> Result<(XHeader, &[u8]), XParseError> {
    if bytes.len() < HEADER_LEN {
        return Err(XParseError::TruncatedHeader(bytes.len()));
    }

    let magic: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if &magic != XOF_MAGIC {
        return Err(XParseError::InvalidMagic(magic));
    }

    let version_str = match std::str::from_utf8(&bytes[4..8]) {
        Ok(s) => s,
        Err(_) => {
            return Err(XParseError::UnsupportedVersion(
                String::from_utf8_lossy(&bytes[4..8]).to_string(),
            ))
        }
    };

    let version = match version_str {
        "0303" => (3, 3),
        "0302" => (3, 2),
        other => return Err(XParseError::UnsupportedVersion(other.to_string())),
    };

    let format_str = match std::str::from_utf8(&bytes[8..12]) {
        Ok(s) => s,
        Err(_) => {
            return Err(XParseError::UnsupportedFormat(
                String::from_utf8_lossy(&bytes[8..12]).to_string(),
            ))
        }
    };

    if format_str != "txt " {
        return Err(XParseError::UnsupportedFormat(format_str.to_string()));
    }

    let float_str = match std::str::from_utf8(&bytes[12..16]) {
        Ok(s) => s,
        Err(_) => {
            return Err(XParseError::UnsupportedFloatSize(
                String::from_utf8_lossy(&bytes[12..16]).to_string(),
            ))
        }
    };

    let float_bits = match float_str {
        "0032" => 32,
        "0064" => 64,
        other => return Err(XParseError::UnsupportedFloatSize(other.to_string())),
    };

    let header = XHeader {
        version,
        float_bits,
    };

    Ok((header, &bytes[HEADER_LEN..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_header_0303_txt_0032() {
        let raw = b"xof 0303txt 0032\r\nFrame Root { }";
        let (header, remainder) = parse_header(raw).expect("header should parse");
        assert_eq!(
            header,
            XHeader {
                version: (3, 3),
                float_bits: 32,
            }
        );
        assert_eq!(remainder, b"\r\nFrame Root { }");
    }

    #[test]
    fn test_valid_header_0302_txt_0064() {
        let raw = b"xof 0302txt 0064payload";
        let (header, remainder) = parse_header(raw).expect("header should parse");
        assert_eq!(
            header,
            XHeader {
                version: (3, 2),
                float_bits: 64,
            }
        );
        assert_eq!(remainder, b"payload");
    }

    #[test]
    fn test_invalid_magic() {
        let raw = b"xoff0303txt 0032";
        let err = parse_header(raw).unwrap_err();
        assert_eq!(err, XParseError::InvalidMagic(*b"xoff"));
    }

    #[test]
    fn test_truncated_header() {
        let raw = b"xof 0303";
        let err = parse_header(raw).unwrap_err();
        assert_eq!(err, XParseError::TruncatedHeader(8));
    }

    #[test]
    fn test_rejects_binary_format() {
        let raw = b"xof 0303bin 0032";
        let err = parse_header(raw).unwrap_err();
        assert_eq!(err, XParseError::UnsupportedFormat("bin ".to_string()));
    }

    #[test]
    fn test_unsupported_version() {
        let raw = b"xof 0304txt 0032";
        let err = parse_header(raw).unwrap_err();
        assert_eq!(err, XParseError::UnsupportedVersion("0304".to_string()));
    }
}
