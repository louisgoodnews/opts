use super::*;

fn file(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&tag.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
    b.extend_from_slice(&(channels * (bits / 8)).to_le_bytes());
    b.extend_from_slice(&bits.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(data);
    b
}

#[test]
fn pcm16_decodes_to_unit_range() {
    let data: Vec<u8> = [0i16, 16384, -32768, 32767]
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let w = decode(&file(1, 2, 44100, 16, &data)).expect("decodes");
    assert_eq!((w.channels, w.sample_rate), (2, 44100));
    assert_eq!(w.samples.len(), 4);
    assert_eq!(w.samples[0], 0.0);
    assert!((w.samples[1] - 0.5).abs() < 1e-6);
    assert_eq!(w.samples[2], -1.0);
    assert!(w.samples[3] < 1.0 && w.samples[3] > 0.999);
    assert!((w.seconds() - 2.0 / 44100.0).abs() < 1e-9);
}

#[test]
fn eight_bit_is_unsigned_around_128() {
    let w = decode(&file(1, 1, 8000, 8, &[128, 255, 0])).expect("decodes");
    assert_eq!(w.samples[0], 0.0);
    assert!(w.samples[1] > 0.99);
    assert_eq!(w.samples[2], -1.0);
}

#[test]
fn pcm24_and_float32_decode() {
    // 24 bit: 0x400000 is half scale.
    let w = decode(&file(
        1,
        1,
        48000,
        24,
        &[0x00, 0x00, 0x40, 0x00, 0x00, 0xC0],
    ))
    .expect("24");
    assert!((w.samples[0] - 0.5).abs() < 1e-6);
    assert!((w.samples[1] + 0.5).abs() < 1e-6);
    let data: Vec<u8> = [0.25f32, -2.0]
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let w = decode(&file(3, 1, 96000, 32, &data)).expect("float");
    assert_eq!(w.samples, vec![0.25, -1.0]);
}

#[test]
fn a_partial_last_frame_and_a_short_data_chunk_are_tolerated() {
    // Stereo with 3 samples: the odd one is dropped.
    let data: Vec<u8> = [1i16, 2, 3].iter().flat_map(|s| s.to_le_bytes()).collect();
    let w = decode(&file(1, 2, 22050, 16, &data)).expect("decodes");
    assert_eq!(w.samples.len(), 2);
    // The data chunk claims more than the file holds.
    let mut b = file(1, 1, 22050, 16, &[1, 0, 2, 0]);
    let at = b.len() - 4 - 4;
    b[at..at + 4].copy_from_slice(&1000u32.to_le_bytes());
    assert_eq!(decode(&b).expect("decodes").samples.len(), 2);
}

#[test]
fn what_is_not_a_wav_is_named() {
    assert_eq!(
        decode(&[0xFF, 0xF2, 0x40, 0xC0, 0, 0, 0, 0, 0, 0, 0, 0]),
        Err(WavError::NotRiff)
    );
    assert_eq!(
        decode(&file(2, 1, 8000, 4, &[0, 0])),
        Err(WavError::Unsupported { tag: 2, bits: 4 })
    );
    let mut no_data = file(1, 1, 8000, 16, &[]);
    no_data.truncate(36);
    assert_eq!(decode(&no_data), Err(WavError::NoData));
}

#[test]
fn odd_sized_chunks_are_padded() {
    let mut b = file(1, 1, 8000, 16, &[]);
    b.truncate(36);
    b.extend_from_slice(b"LIST");
    b.extend_from_slice(&3u32.to_le_bytes());
    b.extend_from_slice(&[1, 2, 3, 0]); // three bytes and the pad byte
    b.extend_from_slice(b"data");
    b.extend_from_slice(&2u32.to_le_bytes());
    b.extend_from_slice(&[0x00, 0x40]);
    let w = decode(&b).expect("decodes");
    assert!((w.samples[0] - 0.5).abs() < 1e-6);
}
