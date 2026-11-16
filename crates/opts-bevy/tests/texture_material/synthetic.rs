//! A minimal synthetic DDS file header with the given parameters.

/// Helper generating a minimal synthetic DDS file header with given parameters.
pub(super) fn build_synthetic_dds_buffer(
    width: u32,
    height: u32,
    fourcc: Option<[u8; 4]>,
    is_a8: bool,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128 + payload.len());
    // Magic 'DDS '
    buf.extend_from_slice(b"DDS ");
    // dwSize = 124
    buf.extend_from_slice(&124u32.to_le_bytes());
    // dwFlags: CAPS | HEIGHT | WIDTH | PIXELFORMAT (0x1007)
    let flags: u32 = 0x1007;
    buf.extend_from_slice(&flags.to_le_bytes());
    // dwHeight, dwWidth
    buf.extend_from_slice(&height.to_le_bytes());
    buf.extend_from_slice(&width.to_le_bytes());
    // dwPitchOrLinearSize
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwDepth
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwMipMapCount
    buf.extend_from_slice(&0u32.to_le_bytes());
    // dwReserved1[11]
    buf.extend_from_slice(&[0u8; 44]);

    // DDS_PIXELFORMAT (32 bytes)
    // dwSize = 32
    buf.extend_from_slice(&32u32.to_le_bytes());
    if let Some(fcc) = fourcc {
        // dwFlags = DDPF_FOURCC (0x4)
        buf.extend_from_slice(&0x4u32.to_le_bytes());
        buf.extend_from_slice(&fcc);
        // dwRGBBitCount, dwRBitMask, dwGBitMask, dwBBitMask, dwABitMask
        buf.extend_from_slice(&[0u8; 20]);
    } else if is_a8 {
        // dwFlags = DDPF_ALPHA (0x2)
        buf.extend_from_slice(&0x2u32.to_le_bytes());
        // dwFourCC = 0
        buf.extend_from_slice(&0u32.to_le_bytes());
        // dwRGBBitCount = 8
        buf.extend_from_slice(&8u32.to_le_bytes());
        // dwRBitMask, dwGBitMask, dwBBitMask = 0
        buf.extend_from_slice(&[0u8; 12]);
        // dwABitMask = 0xFF
        buf.extend_from_slice(&0xFFu32.to_le_bytes());
    } else {
        // Uncompressed RGBA (32-bit)
        // dwFlags = DDPF_RGB | DDPF_ALPHAPIXELS (0x41)
        buf.extend_from_slice(&0x41u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&32u32.to_le_bytes());
        buf.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // R
        buf.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // G
        buf.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // B
        buf.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // A
    }

    // dwCaps: DDSCAPS_TEXTURE (0x1000)
    buf.extend_from_slice(&0x1000u32.to_le_bytes());
    // dwCaps2, dwCaps3, dwCaps4, dwReserved2
    buf.extend_from_slice(&[0u8; 16]);

    // Append pixel data payload
    buf.extend_from_slice(payload);
    buf
}
