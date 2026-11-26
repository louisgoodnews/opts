//! Deterministic fuzz-style tests for the untrusted readers of `.x`, `.bus`, `.sli` and `.hof`
//! (text), the map tile and terrain (`.map`, `.map.terrain`) and `.o3d` (binary) (ISS-006, with
//! the count bound of ISS-005).
//!
//! No external crate: a fixed-seed xorshift64 (`fuzz_parsers/mutate.rs`) drives every mutation, so
//! each run sees the same inputs. Each reader is fed (a) every prefix of a small valid sample,
//! (b) every digit run of a text sample replaced by a huge, negative or malformed number, or for a
//! binary sample every `u32`/`u16` window replaced by an edge word (counts are fixed-width there),
//! and (c) seeded random byte and token mutations. Every input must return `Ok` or `Err` without
//! panicking, and no single allocation may exceed a bound derived from the input length. That
//! bound is the ISS-005 rule in observable form: a file-supplied count must not allocate beyond
//! the bytes it came from.
//!
//! The counting allocator (`fuzz_parsers/alloc.rs`) needs `unsafe` (the workspace denies
//! `unsafe_code`); it only forwards to `System` and records the largest request.

#[path = "fuzz_parsers/alloc.rs"]
#[allow(unsafe_code)] // the counting allocator is the only unsafe code in the test
mod alloc;
#[path = "fuzz_parsers/mutate.rs"]
mod mutate;
#[path = "fuzz_parsers/readers.rs"]
mod readers;
#[path = "fuzz_parsers/samples.rs"]
mod samples;

use std::panic::{self, AssertUnwindSafe};
use std::sync::Mutex;

use mutate::{digit_spans, mutate, windows, with_number_replaced, XorShift, NUMBERS};
use readers::{
    Reader, BUS_READER, HOF_READER, MAP_TILE_READER, O3D_READER, SLI_READER, TERRAIN_READER,
    X_READER,
};

#[global_allocator]
static GLOBAL: alloc::PeakAlloc = alloc::PeakAlloc;

/// Serialises the tests of this binary, so the peak counter sees one parse at a time.
static SERIAL: Mutex<()> = Mutex::new(());

const ITERATIONS: usize = 1500;

/// Largest allocation a parse of `len` input bytes may request. Every token takes at least one
/// input byte, every list element needs at least one token, and a token or element costs a few
/// dozen bytes of storage; the constant covers the fixed setup (a token vector's growth, a line
/// table). A count that allocated from its declared value would exceed this for a huge count.
fn allocation_limit(len: usize) -> usize {
    64 * len + 4096
}

/// Runs one input through the reader. A panic or an allocation over the bound fails the test.
fn check(reader: &Reader, input: &[u8], context: &str) {
    alloc::reset_peak();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| (reader.parse)(input)));
    let peak = alloc::peak();
    assert!(
        outcome.is_ok(),
        "{}: panicked on {context}: {:?}",
        reader.name,
        String::from_utf8_lossy(input)
    );
    let limit = allocation_limit(input.len());
    assert!(
        peak <= limit,
        "{}: {context}: a single allocation of {peak} bytes exceeds the {limit}-byte bound \
         for a {}-byte input (a file-supplied count allocated beyond the input): {:?}",
        reader.name,
        input.len(),
        String::from_utf8_lossy(input)
    );
}

fn fuzz_reader(reader: &Reader, seed: u64) {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let sample = (reader.sample)();
    assert!(
        (reader.parse)(&sample),
        "{}: the full sample must parse",
        reader.name
    );

    for end in 0..=sample.len() {
        check(reader, &sample[..end], &format!("prefix of length {end}"));
    }

    if reader.binary {
        for (index, input) in windows(&sample).iter().enumerate() {
            check(reader, input, &format!("window replacement {index}"));
        }
    } else {
        for (index, span) in digit_spans(&sample).into_iter().enumerate() {
            for replacement in NUMBERS {
                let input = with_number_replaced(&sample, span, replacement);
                check(
                    reader,
                    &input,
                    &format!("digit run {index} replaced by {replacement:?}"),
                );
            }
        }
    }

    let mut rng = XorShift::new(seed);
    for iteration in 0..ITERATIONS {
        let input = mutate(&sample, &mut rng);
        check(reader, &input, &format!("random mutation {iteration}"));
    }
}

#[test]
fn x_survives_prefixes_and_mutations_within_the_count_bound() {
    fuzz_reader(&X_READER, 0x9E37_79B9_7F4A_7C15);
}

#[test]
fn bus_survives_prefixes_and_mutations() {
    fuzz_reader(&BUS_READER, 0xC2B2_AE3D_27D4_EB4F);
}

#[test]
fn sli_survives_prefixes_and_mutations() {
    fuzz_reader(&SLI_READER, 0x1656_67B1_9E37_79F9);
}

#[test]
fn hof_survives_prefixes_and_mutations() {
    fuzz_reader(&HOF_READER, 0x27D4_EB2F_1656_67C5);
}

#[test]
fn map_tile_survives_prefixes_and_mutations() {
    fuzz_reader(&MAP_TILE_READER, 0x94D0_49BB_1331_11EB);
}

#[test]
fn o3d_survives_prefixes_windows_and_mutations_within_the_count_bound() {
    fuzz_reader(&O3D_READER, 0xBF58_476D_1CE4_E5B9);
}

#[test]
fn map_terrain_survives_prefixes_windows_and_mutations() {
    fuzz_reader(&TERRAIN_READER, 0x2545_F491_4F6C_DD1D);
}
