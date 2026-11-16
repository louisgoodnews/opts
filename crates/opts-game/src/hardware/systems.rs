//! The probe's systems: the start, the poll of a finished probe, and the renderer's facts handed
//! to the next probe once the window system has them.

use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;
use bevy::render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy::window::{Monitor, PrimaryMonitor};
use opts_hardware::inputs::{DisplayInput, GpuInput, InputDevicesInput};

use super::{data_dir, HardwareState, RENDERER_WAIT_FRAMES};

pub(super) fn start(mut state: ResMut<HardwareState>) {
    if state.dir.is_none() {
        state.dir = data_dir();
    }
    state.start_probe(false, false);
}

pub(super) fn finish(
    mut state: ResMut<HardwareState>,
    page: Option<ResMut<crate::settings_screen::SettingsPageState>>,
) {
    if state.poll() == Some(true) {
        // Rebuild the page so the new facts show.
        if let Some(mut p) = page {
            p.dirty = true;
        }
    }
}

/// Hands renderer facts to the probe once they exist, then probes again.
pub(super) fn collect_renderer_inputs(
    mut state: ResMut<HardwareState>,
    adapter: Option<Res<RenderAdapterInfo>>,
    device: Option<Res<RenderDevice>>,
    monitors: Query<(&Monitor, Has<PrimaryMonitor>)>,
    pads: Query<(), With<Gamepad>>,
) {
    if state.renderer_applied {
        return;
    }
    state.frames += 1;
    let have_monitor = !monitors.is_empty();
    if !(adapter.is_some() && have_monitor) && state.frames < RENDERER_WAIT_FRAMES {
        return;
    }
    state.renderer_applied = true;
    if let Some(a) = adapter {
        let mut gpu = GpuInput {
            vendor_id: a.vendor,
            device_type: format!("{:?}", a.device_type),
            backend: format!("{:?}", a.backend),
            ..GpuInput::default()
        };
        if let Some(d) = device {
            let limits = d.limits();
            gpu.max_texture_2d = Some(limits.max_texture_dimension_2d);
            gpu.max_buffer_bytes = Some(limits.max_buffer_size);
            gpu.features = d
                .features()
                .iter_names()
                .map(|(n, _)| n.to_ascii_lowercase())
                .collect();
            gpu.ray_tracing = Some(gpu.features.iter().any(|f| f.contains("ray_query")));
        }
        state.inputs.gpu = Some(gpu);
    }
    if have_monitor {
        let primary = monitors
            .iter()
            .find(|(_, p)| *p)
            .or_else(|| monitors.iter().next())
            .map(|(m, _)| m);
        state.inputs.display = primary.map(|m| DisplayInput {
            count: u32::try_from(monitors.iter().count()).unwrap_or(u32::MAX),
            width: m.physical_width,
            height: m.physical_height,
            refresh_millihertz: m.refresh_rate_millihertz,
            scale_factor: Some(m.scale_factor),
            hdr: None,
        });
    }
    state.inputs.input = Some(InputDevicesInput {
        gamepads: u32::try_from(pads.iter().count()).unwrap_or(u32::MAX),
        wheels: None,
    });
    state.start_probe(false, false);
}
