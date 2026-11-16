//! Reading a cache file: its header, its checksum and whether it fits the key.

use super::*;

/// FNV-1a over the payload: catches truncated or damaged files, not attacks.
pub(super) fn checksum(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The chain and level count of a cache file, when its header and checksum agree with `key`.
pub(super) fn parse<'a>(bytes: &'a [u8], key: &ChainKey) -> Option<(&'a [u8], u32)> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return None;
    }
    let word =
        |i: usize| u32::from_le_bytes(bytes[8 + 4 * i..12 + 4 * i].try_into().unwrap_or_default());
    let (generator, format, width, height, levels) = (word(0), word(1), word(2), word(3), word(4));
    let len = u64::from_le_bytes(bytes[28..36].try_into().unwrap_or_default());
    let sum = u64::from_le_bytes(bytes[36..44].try_into().unwrap_or_default());
    let payload = &bytes[HEADER..];
    let expected = chain_bytes(width, height, levels, key.block_bytes());
    if (generator, format, width, height) != (key.generator, key.format, key.width, key.height)
        || levels != full_chain_levels(width, height)
        || len != payload.len() as u64
        || payload.len() != expected
        || checksum(payload) != sum
    {
        return None;
    }
    Some((payload, levels))
}
