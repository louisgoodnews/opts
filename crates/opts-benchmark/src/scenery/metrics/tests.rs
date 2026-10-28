//! Unit tests for `super` (`metrics.rs`).

use super::*;

fn buffers(object: &[usize], colour: [u8; 3]) -> (Vec<u8>, Vec<u8>) {
    let n = (IMAGE_SIZE * IMAGE_SIZE) as usize;
    let (mut b, mut w) = (vec![0u8; n * 4], vec![255u8; n * 4]);
    for c in w.chunks_exact_mut(4) {
        c[3] = 255;
    }
    for &i in object {
        b[i * 4..i * 4 + 3].copy_from_slice(&colour);
        w[i * 4..i * 4 + 3].copy_from_slice(&colour);
    }
    (b, w)
}

#[test]
fn matting_finds_object_pixels_of_any_colour() {
    for colour in [[0, 0, 0], [255, 255, 255], [255, 0, 255], [30, 200, 90]] {
        let (b, w) = buffers(&[10, 11, 600], colour);
        let m = matte(&b, &w);
        assert_eq!(m.mask.count(), 3);
        assert_eq!(m.rgb[10], colour);
        assert_eq!(
            m.sentinel.count(),
            if colour == [255, 0, 255] { 3 } else { 0 }
        );
    }
}

#[test]
fn changed_share_sees_moved_glyphs_and_ignores_empty_background() {
    let region = Mask {
        width: 512,
        height: 512,
        px: vec![true; 512 * 512],
    };
    let (b1, w1) = buffers(&[10, 11], [0, 0, 0]);
    let (b2, w2) = buffers(&[12, 13], [0, 0, 0]);
    let (ma, mb) = (matte(&b1, &w1), matte(&b2, &w2));
    assert_eq!(changed_share(&ma, &mb, &region, 40), Some(1.0));
    assert_eq!(changed_share(&ma, &ma, &region, 40), Some(0.0));
    let (b0, w0) = buffers(&[], [0, 0, 0]);
    let blank = matte(&b0, &w0);
    assert_eq!(changed_share(&blank, &blank, &region, 40), Some(0.0));
    let empty = Mask::new(512, 512);
    assert_eq!(changed_share(&ma, &mb, &empty, 40), None);
}

#[test]
fn back_faces_are_not_expected_and_billboards_are_two_sided() {
    use super::super::expected::TriFlags;
    use bevy::math::Vec3;
    let view = super::super::framing::top_view(&super::super::framing::Bounds {
        min: Vec3::new(-2.0, -2.0, 0.0),
        max: Vec3::new(2.0, 2.0, 1.0),
    });
    let place = Placement {
        position: Vec3::ZERO,
        heading_deg: 0.0,
        mirror_x: false,
        scale: 1.0,
    };
    let tri = |v: [Vec3; 3], two_sided| ExpectedTri {
        v,
        flags: TriFlags {
            base: true,
            two_sided,
            ..TriFlags::default()
        },
        mesh: 0,
    };
    let a = [
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(-1.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, -1.0),
    ];
    let b = [a[0], a[2], a[1]];
    let front = expected_masks(&[tri(a, false)], &place, &view).all.count();
    let back = expected_masks(&[tri(b, false)], &place, &view).all.count();
    assert!(
        (front == 0) != (back == 0),
        "exactly one winding faces the camera"
    );
    let both = expected_masks(&[tri(a, true)], &place, &view).all.count();
    assert!(both > 0);
    let both_b = expected_masks(&[tri(b, true)], &place, &view).all.count();
    assert_eq!(both, both_b);
}

#[test]
fn identical_masks_give_perfect_metrics() {
    let (b, w) = buffers(&(1000..1100).collect::<Vec<_>>(), [90, 90, 90]);
    let r = matte(&b, &w);
    let e = ExpectedMasks {
        all: r.mask.clone(),
        base: r.mask.clone(),
        solid: r.mask.clone(),
        text: Mask::new(512, 512),
    };
    let m = view_metrics(&r, &e);
    assert_eq!(m.precision, Some(1.0));
    assert_eq!(m.recall, Some(1.0));
    assert_eq!(m.iou, Some(1.0));
    assert_eq!(m.bbox_overshoot, Some(0.0));
    assert_eq!(m.size_ratio, Some([1.0, 1.0]));
}
