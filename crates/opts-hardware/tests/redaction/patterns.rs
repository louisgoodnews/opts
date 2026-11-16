//! Structural scanners for MAC addresses and UUIDs in serialised text.

pub(super) fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `aa:bb:cc:dd:ee:ff` or `aa-bb-cc-dd-ee-ff`.
pub(super) fn has_mac(text: &str) -> bool {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(16)).any(|i| {
        let w = &text[i..i + 17];
        w.is_ascii()
            && [2, 5, 8, 11, 14]
                .iter()
                .all(|k| matches!(w.as_bytes()[*k], b':' | b'-'))
            && [0, 3, 6, 9, 12, 15].iter().all(|k| is_hex(&w[*k..*k + 2]))
    })
}

/// 8-4-4-4-12 hex groups.
pub(super) fn has_uuid(text: &str) -> bool {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(35)).any(|i| {
        let w = &text[i..i + 36];
        w.is_ascii()
            && [8, 13, 18, 23].iter().all(|k| w.as_bytes()[*k] == b'-')
            && [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)]
                .iter()
                .all(|(a, z)| is_hex(&w[*a..*z]))
    })
}
