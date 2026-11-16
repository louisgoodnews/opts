//! Unit tests for `super` (`audio_content.rs`).

use super::*;

fn wav_bytes() -> Vec<u8> {
    let data: Vec<u8> = [0i16, 8192, -8192, 0]
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&22050u32.to_le_bytes());
    b.extend_from_slice(&44100u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    b
}

#[test]
fn a_file_is_decoded_once_and_shared() {
    let dir = std::env::temp_dir().join(format!("opts-clip-cache-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let good = dir.join("beep.wav");
    let bad = dir.join("mpeg.wav");
    std::fs::write(&good, wav_bytes()).expect("write");
    std::fs::write(&bad, [0xFF, 0xF2, 0x40, 0xC0, 1, 2, 3, 4, 5, 6, 7, 8]).expect("write");
    let mut assets = Assets::<PcmClip>::default();
    let mut cache = ClipCache::default();
    let a = cache.clip(&good, &mut assets).expect("decodes");
    let b = cache.clip(&good, &mut assets).expect("cached");
    assert_eq!(a, b);
    assert_eq!(cache.len(), 1);
    let clip = assets.get(&a).expect("asset");
    assert_eq!(
        (clip.channels, clip.sample_rate, clip.frames()),
        (1, 22050, 4)
    );
    // A file that is not RIFF/WAVE is refused with a reason and remembered.
    let e = cache.clip(&bad, &mut assets).expect_err("refused");
    assert!(e.contains("not a RIFF/WAVE"), "{e}");
    let e2 = cache.clip(&bad, &mut assets).expect_err("refused again");
    assert!(e2.contains("failed before"), "{e2}");
    assert!(cache.clip(&dir.join("none.wav"), &mut assets).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_loader_decodes_on_a_worker_and_reports_states() {
    let dir = std::env::temp_dir().join(format!("opts-clip-loader-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let good = dir.join("beep.wav");
    std::fs::write(&good, wav_bytes()).expect("write");
    let bad = dir.join("none.wav");
    let mut assets = Assets::<PcmClip>::default();
    let mut loader = ClipLoader::default();
    assert_eq!(loader.state(&good), ClipState::Loading);
    assert_eq!(loader.state(&bad), ClipState::Loading);
    for _ in 0..200 {
        loader.poll(&mut assets);
        if loader.counts().1 == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(loader.counts(), (1, 0, 1));
    let ClipState::Ready(h) = loader.state(&good) else {
        panic!("the good file is ready");
    };
    assert_eq!(assets.get(&h).map(PcmClip::frames), Some(4));
    assert_eq!(loader.state(&bad), ClipState::Failed);
    assert_eq!(loader.bytes, 16);
    let _ = std::fs::remove_dir_all(&dir);
}
