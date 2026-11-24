//! Fixed-seed xorshift and the byte and token mutations applied to the valid samples.

/// Bytes that carry syntax in the four formats; inserted at random positions.
const SYNTAX: &[u8] = b"{};,[]-.09eE\n\t \0\xff";

/// Replacements for a digit run: huge counts (including the `u32` edges), a count past `u32`,
/// negative, empty and malformed numbers.
pub const NUMBERS: &[&str] = &[
    "4000000000",
    "4294967295",
    "4294967296",
    "99999999999999999999",
    "50000000",
    "-1",
    "0",
    "",
    "NaN",
    "1e308",
];

/// Fixed-seed xorshift64; the seed is never zero.
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Start and end of every run of ASCII digits.
pub fn digit_spans(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            spans.push((start, i));
        } else {
            i += 1;
        }
    }
    spans
}

/// The sample with one digit run replaced by `replacement`.
pub fn with_number_replaced(sample: &[u8], span: (usize, usize), replacement: &str) -> Vec<u8> {
    let mut out = sample.to_vec();
    out.splice(span.0..span.1, replacement.bytes());
    out
}

/// One to three random byte or token mutations of `sample`.
pub fn mutate(sample: &[u8], rng: &mut XorShift) -> Vec<u8> {
    let mut out = sample.to_vec();
    for _ in 0..1 + rng.below(3) {
        match rng.below(4) {
            0 if !out.is_empty() => {
                let i = rng.below(out.len());
                out[i] = rng.next_u64() as u8;
            }
            1 if !out.is_empty() => {
                let i = rng.below(out.len());
                let end = (i + 1 + rng.below(8)).min(out.len());
                out.drain(i..end);
            }
            2 => {
                let i = rng.below(out.len() + 1);
                out.insert(i, SYNTAX[rng.below(SYNTAX.len())]);
            }
            _ => {
                let spans = digit_spans(&out);
                if !spans.is_empty() {
                    let span = spans[rng.below(spans.len())];
                    let replacement = NUMBERS[rng.below(NUMBERS.len())];
                    out.splice(span.0..span.1, replacement.bytes());
                }
            }
        }
    }
    out
}

/// Little-endian words that stand for counts and sizes at the edges: `u32` and `u16` extremes,
/// the 65,536 wrap of a version 1 `.o3d` count, and the `+inf` and NaN float patterns.
const WINDOW_U32: [u32; 7] = [
    0xFFFF_FFFF,
    0x8000_0000,
    0x0001_0000,
    0x7FFF_FFFF,
    4_000_000_000,
    0x7F80_0000,
    0x7FC0_0000,
];
const WINDOW_U16: [u16; 4] = [0xFFFF, 0x8000, 0x0100, 0x0200];

/// For a binary sample: every `u32` and `u16` window of the sample overwritten by each edge word.
/// This is the binary counterpart of the digit-run replacement, since counts there are fixed-width
/// fields rather than digit runs.
pub fn windows(sample: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for offset in 0..sample.len() {
        for word in WINDOW_U32 {
            if offset + 4 <= sample.len() {
                let mut input = sample.to_vec();
                input[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
                out.push(input);
            }
        }
        for word in WINDOW_U16 {
            if offset + 2 <= sample.len() {
                let mut input = sample.to_vec();
                input[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
                out.push(input);
            }
        }
    }
    out
}
