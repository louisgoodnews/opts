//! Header parser for `.o3d` files.
//!
//! Corresponds to `docs/src/formats/binary/o3d.md#header` and
//! `docs/src/formats/binary/o3d-chunks.md#header`.

use std::io::{Cursor, Read};

use crate::formats::o3d::error::{O3dParseError, Result};

/// Magic bytes constant identifying a valid `.o3d` file (`[0x84, 0x19]`).
pub const O3D_MAGIC: [u8; 2] = [0x84, 0x19];

/// Supported `.o3d` version numbers observed and verified in the corpus.
pub const SUPPORTED_VERSIONS: [u8; 5] = [1, 3, 4, 5, 7];

/// Parsed header of an `.o3d` mesh file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// File format version (1, 3, 4, 5, or 7).
    pub version: u32,
    /// Bitflags byte (present in version >= 3, defaults to 0 for v1).
    pub flags: u8,
    /// Opaque metadata stamp (present in version >= 4).
    pub stamp: Option<u32>,
}

impl Header {
    /// Returns `true` if bit 0 (`0x01`) of the flags byte is set,
    /// indicating 32-bit vertex indices instead of 16-bit indices.
    #[must_use]
    pub fn is_32bit_indices(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Returns `true` if count fields (vertex count and face count)
    /// are encoded as `u32` (version >= 3). Version 1 uses `u16`.
    #[must_use]
    pub fn is_32bit_counts(&self) -> bool {
        self.version >= 3
    }

    /// Parses an `.o3d` header from a cursor over a byte buffer.
    ///
    /// # Errors
    ///
    /// Returns [`O3dParseError::Truncated`] if the cursor contains insufficient bytes.
    /// Returns [`O3dParseError::InvalidMagic`] if the first two bytes are not `[0x84, 0x19]`.
    /// Returns [`O3dParseError::UnsupportedVersion`] if the version is not in `[1, 3, 4, 5, 7]`.
    pub fn parse(reader: &mut Cursor<&[u8]>) -> Result<Self> {
        let initial_pos = reader.position() as usize;
        let total_len = reader.get_ref().len();

        let mut magic = [0u8; 2];
        read_exact_bytes(reader, &mut magic, initial_pos, total_len, 2)?;

        if magic != O3D_MAGIC {
            return Err(O3dParseError::InvalidMagic(magic[0], magic[1]));
        }

        let mut ver_buf = [0u8; 1];
        read_exact_bytes(reader, &mut ver_buf, initial_pos, total_len, 3)?;
        let version = ver_buf[0];

        if !SUPPORTED_VERSIONS.contains(&version) {
            return Err(O3dParseError::UnsupportedVersion(u32::from(version)));
        }

        match version {
            1 => Ok(Self {
                version: 1,
                flags: 0,
                stamp: None,
            }),
            3 => {
                let mut flags_buf = [0u8; 1];
                read_exact_bytes(reader, &mut flags_buf, initial_pos, total_len, 4)?;
                Ok(Self {
                    version: 3,
                    flags: flags_buf[0],
                    stamp: None,
                })
            }
            4..=u8::MAX => {
                let mut flags_buf = [0u8; 1];
                read_exact_bytes(reader, &mut flags_buf, initial_pos, total_len, 4)?;

                let mut stamp_buf = [0u8; 4];
                read_exact_bytes(reader, &mut stamp_buf, initial_pos, total_len, 8)?;
                let stamp = u32::from_le_bytes(stamp_buf);

                Ok(Self {
                    version: u32::from(version),
                    flags: flags_buf[0],
                    stamp: Some(stamp),
                })
            }
            0 | 2 => unreachable!("checked by SUPPORTED_VERSIONS"),
        }
    }
}

/// Helper function to read exact bytes or return a Truncated error.
fn read_exact_bytes(
    reader: &mut Cursor<&[u8]>,
    buf: &mut [u8],
    _start_pos: usize,
    total_len: usize,
    expected_min: usize,
) -> Result<()> {
    reader.read_exact(buf).map_err(|_| {
        let actual = total_len.saturating_sub(reader.position() as usize);
        O3dParseError::Truncated {
            expected: expected_min,
            actual,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_v1_header() {
        let bytes = [0x84, 0x19, 0x01, 0x17];
        let mut cursor = Cursor::new(&bytes[..]);
        let header = Header::parse(&mut cursor).expect("v1 header should parse");
        assert_eq!(header.version, 1);
        assert_eq!(header.flags, 0);
        assert_eq!(header.stamp, None);
        assert!(!header.is_32bit_indices());
        assert!(!header.is_32bit_counts());
        assert_eq!(cursor.position(), 3);
    }

    #[test]
    fn test_valid_v3_header() {
        let bytes = [0x84, 0x19, 0x03, 0x01];
        let mut cursor = Cursor::new(&bytes[..]);
        let header = Header::parse(&mut cursor).expect("v3 header should parse");
        assert_eq!(header.version, 3);
        assert_eq!(header.flags, 1);
        assert_eq!(header.stamp, None);
        assert!(header.is_32bit_indices());
        assert!(header.is_32bit_counts());
        assert_eq!(cursor.position(), 4);
    }

    #[test]
    fn test_valid_v7_header() {
        let bytes = [0x84, 0x19, 0x07, 0x02, 0x78, 0x56, 0x34, 0x12];
        let mut cursor = Cursor::new(&bytes[..]);
        let header = Header::parse(&mut cursor).expect("v7 header should parse");
        assert_eq!(header.version, 7);
        assert_eq!(header.flags, 2);
        assert_eq!(header.stamp, Some(0x1234_5678));
        assert!(!header.is_32bit_indices());
        assert!(header.is_32bit_counts());
        assert_eq!(cursor.position(), 8);
    }

    #[test]
    fn test_truncated_header() {
        let bytes = [0x84, 0x19];
        let mut cursor = Cursor::new(&bytes[..]);
        let err = Header::parse(&mut cursor).unwrap_err();
        assert!(matches!(err, O3dParseError::Truncated { .. }));

        let v7_truncated = [0x84, 0x19, 0x07, 0x00, 0x01, 0x02];
        let mut cursor = Cursor::new(&v7_truncated[..]);
        let err = Header::parse(&mut cursor).unwrap_err();
        assert!(matches!(err, O3dParseError::Truncated { .. }));
    }

    #[test]
    fn test_invalid_magic() {
        let bytes = [0x00, 0x00, 0x01];
        let mut cursor = Cursor::new(&bytes[..]);
        let err = Header::parse(&mut cursor).unwrap_err();
        assert_eq!(err, O3dParseError::InvalidMagic(0x00, 0x00));
    }

    #[test]
    fn test_unsupported_version() {
        let bytes = [0x84, 0x19, 0x02];
        let mut cursor = Cursor::new(&bytes[..]);
        let err = Header::parse(&mut cursor).unwrap_err();
        assert_eq!(err, O3dParseError::UnsupportedVersion(2));

        let bytes_v8 = [0x84, 0x19, 0x08, 0x00];
        let mut cursor = Cursor::new(&bytes_v8[..]);
        let err = Header::parse(&mut cursor).unwrap_err();
        assert_eq!(err, O3dParseError::UnsupportedVersion(8));
    }
}
