//! Robustness of the `.o3d` parser: empty, short, pseudo-random and truncated inputs.

use super::*;

#[test]
fn test_robustness_empty_and_short_inputs() {
    // 1. Empty byte slice
    assert!(parse(&[]).is_err());

    // 2. Single-byte inputs across all possible values
    for b in 0..=u8::MAX {
        assert!(parse(&[b]).is_err());
    }

    // 3. Short prefixes of magic
    assert!(parse(&[0x84]).is_err());
    assert!(parse(&[0x84, 0x19]).is_err());
}

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    // Deterministic 64-bit Xorshift PRNG
    let mut state: u64 = 0xCAFE_BABE_DEAD_BEEF;
    let mut xorshift64 = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    // Test 100 buffers of 512 pseudorandom bytes
    for _ in 0..100 {
        let mut buffer = [0u8; 512];
        for chunk in buffer.chunks_exact_mut(8) {
            chunk.copy_from_slice(&xorshift64().to_le_bytes());
        }
        let _ = parse(&buffer); // Must not panic
    }
}

#[test]
fn test_robustness_prefix_truncation() {
    let bytes = match load_fixture("t04_a_identity.o3d") {
        Some(b) => b,
        None => return,
    };

    // For every prefix from 1 byte up to len - 1:
    // It must NEVER panic. If it succeeds, it can only be at the optional chunk boundary (offset 939).
    for len in 1..bytes.len() {
        let result = parse(&bytes[..len]);
        if len != 939 {
            assert!(
                result.is_err(),
                "prefix of length {len} should fail, but succeeded"
            );
        } else {
            // Offset 939 is the valid end of materials before the optional 0x79 matrix tag
            assert!(result.is_ok());
        }
    }
}
