//! Fitting the pictures and captions of the door panels to the common width and the slots' fades.

use super::spawn::caption;
use super::*;

pub(super) type Images<'w, 's> = Query<
    'w,
    's,
    (
        &'static DoorOverlayImage,
        &'static mut Node,
        &'static mut ImageNode,
    ),
    Without<DoorOverlay>,
>;
pub(super) type Captions<'w, 's> = Query<
    'w,
    's,
    (
        &'static DoorOverlayCaption,
        &'static mut Text,
        &'static mut InstrumentFade,
    ),
    Without<DoorOverlay>,
>;

/// Every picture takes the common width and the fade of its slot.
pub(super) fn fit_images(images: &mut Images, slots: &[DoorSlot], width: f32) {
    for (marker, mut node, mut image) in images {
        let w = Val::Px(width);
        if node.width != w {
            node.width = w;
        }
        if let Some(slot) = slots.iter().find(|s| s.door == marker.0) {
            let color = image.color.with_alpha(slot.fade);
            if image.color != color {
                image.color = color;
            }
        }
    }
}

/// Every caption names its door and fades with its slot.
pub(super) fn fit_captions(
    captions: &mut Captions,
    slots: &[DoorSlot],
    doors: Option<&DoorPoints>,
    loc: &Localization,
) {
    for (marker, mut text, mut fade) in captions {
        let Some(slot) = slots.iter().find(|s| s.door == marker.0) else {
            continue;
        };
        let words = doors.map_or_else(String::new, |d| caption(loc, d, marker.0));
        if text.0 != words {
            text.0 = words;
        }
        if fade.0 != slot.fade {
            fade.0 = slot.fade;
        }
    }
}
