//! The multi-line blocks of a bus file: axles, mirror cameras and driver and passenger cameras.

use super::*;

pub(super) fn first_float(params: &[String]) -> Option<f32> {
    params.first().and_then(|p| parse_float(p).ok())
}

pub(super) fn extract_counted_list(params: &[String], dest: &mut Vec<String>) {
    if params.is_empty() {
        return;
    }
    // Line 1 is the declared count N
    let count = params[0].parse::<usize>().unwrap_or(params.len() - 1);
    for item in params.iter().skip(1).take(count) {
        dest.push(item.clone());
    }
}

pub(super) fn parse_axle_block(params: &[String], axle: &mut BusAxle) {
    let mut iter = params.iter();
    while let Some(ident) = iter.next() {
        match ident.as_str() {
            "achse_long" => {
                if let Some(val) = iter.next() {
                    axle.longitudinal_m = parse_float(val).unwrap_or(0.0);
                }
            }
            "achse_maxwidth" => {
                if let Some(val) = iter.next() {
                    axle.max_width_m = parse_float(val).ok();
                }
            }
            "achse_minwidth" => {
                if let Some(val) = iter.next() {
                    axle.min_width_m = parse_float(val).ok();
                }
            }
            "achse_raddurchmesser" => {
                if let Some(val) = iter.next() {
                    axle.wheel_diameter_m = parse_float(val).ok();
                }
            }
            "achse_feder" => {
                if let Some(val) = iter.next() {
                    axle.spring_rate = parse_float(val).ok();
                }
            }
            "achse_maxforce" => {
                if let Some(val) = iter.next() {
                    axle.max_force = parse_float(val).ok();
                }
            }
            "achse_daempfer" => {
                if let Some(val) = iter.next() {
                    axle.damping = parse_float(val).ok();
                }
            }
            "achse_antrieb" => {
                if let Some(val) = iter.next() {
                    axle.is_driven = val == "1" || val.eq_ignore_ascii_case("true");
                }
            }
            _ => {}
        }
    }
}

pub(super) fn parse_mirror_block(params: &[String]) -> Option<BusMirrorCamera> {
    if params.len() < 7 {
        return None;
    }
    let p = |i: usize| parse_float(&params[i]).ok();
    Some(BusMirrorCamera {
        x: p(0)?,
        y: p(1)?,
        z: p(2)?,
        param4: p(3)?,
        fov_deg: p(4)?,
        yaw_deg: p(5)?,
        pitch_deg: p(6)?,
        param8: params.get(7).and_then(|_| p(7)),
    })
}

pub(super) fn parse_camera_block(params: &[String], is_driver: bool) -> Option<BusCamera> {
    if params.len() < 7 {
        return None;
    }
    let x = parse_float(&params[0]).ok()?;
    let y = parse_float(&params[1]).ok()?;
    let z = parse_float(&params[2]).ok()?;
    let dist = parse_float(&params[3]).ok()?;
    let fov = parse_float(&params[4]).ok()?;
    let yaw = parse_float(&params[5]).ok()?;
    let pitch = parse_float(&params[6]).ok()?;

    Some(BusCamera {
        x,
        y,
        z,
        dist,
        fov_deg: fov,
        yaw_deg: yaw,
        pitch_deg: pitch,
        is_driver,
        is_schedule_view: false,
        is_ticket_view: false,
    })
}
