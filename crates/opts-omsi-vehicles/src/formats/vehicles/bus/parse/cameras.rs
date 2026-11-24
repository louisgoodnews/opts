//! Cameras of a `.bus` manifest: driver, passenger and mirror cameras and the camera settings.

use super::super::blocks::{parse_camera_block, parse_mirror_block};
use super::super::*;
use crate::formats::text::OmsiBlock;

pub(super) fn handle_cameras(block: &OmsiBlock, m: &mut BusManifest) {
    match block.keyword.as_str() {
        "add_camera_driver" if block.parameters.len() >= 7 => {
            if let Some(cam) = parse_camera_block(&block.parameters, true) {
                m.driver_cameras.push(cam);
            }
        }
        "add_camera_pax" if block.parameters.len() >= 7 => {
            if let Some(cam) = parse_camera_block(&block.parameters, false) {
                m.pax_cameras.push(cam);
            }
        }
        "add_camera_reflexion" | "add_camera_reflexion_2" => {
            if let Some(cam) = parse_mirror_block(&block.parameters) {
                m.mirror_cameras.push(cam);
            }
        }
        "set_camera_std" => {
            if let Some(first) = block.parameters.first() {
                m.std_camera_index = first.trim().parse::<usize>().ok();
            }
        }
        "set_camera_outside_center" if block.parameters.len() >= 3 => {
            let x = parse_float(&block.parameters[0]).unwrap_or(0.0);
            let y = parse_float(&block.parameters[1]).unwrap_or(0.0);
            let z = parse_float(&block.parameters[2]).unwrap_or(0.0);
            m.outside_camera_center = Some(glam::Vec3::new(x, y, z));
        }
        "view_schedule" => {
            if let Some(cam) = m.driver_cameras.last_mut() {
                cam.is_schedule_view = true;
            }
        }
        "view_ticketselling" => {
            if let Some(cam) = m.driver_cameras.last_mut() {
                cam.is_ticket_view = true;
            }
        }
        _ => {}
    }
}
