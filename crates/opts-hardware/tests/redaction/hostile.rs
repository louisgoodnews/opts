//! Probe inputs carrying identifiers in every free-text field.

use opts_hardware::inputs::{AudioInput, DisplayInput, GpuInput, InputDevicesInput, ProbeInputs};

pub(super) fn hostile() -> ProbeInputs {
    let evil = "AA:BB:CC:DD:EE:FF 123e4567-e89b-12d3-a456-426614174000 my-hostname louis";
    ProbeInputs {
        gpu: Some(GpuInput {
            vendor_id: 0x10de,
            device_type: format!("DiscreteGpu {evil}"),
            backend: format!("Vulkan {evil}"),
            vram_mib: Some(8192),
            max_texture_2d: Some(16384),
            max_buffer_bytes: Some(1 << 32),
            features: vec![
                evil.to_owned(),
                "texture_compression_bc".to_owned(),
                format!("{evil}timestamp_query"),
            ],
            ray_tracing: Some(true),
            dlss_runtime: Some(false),
        }),
        display: Some(DisplayInput {
            count: 2,
            width: 2560,
            height: 1440,
            refresh_millihertz: Some(143_981),
            scale_factor: Some(1.0),
            hdr: Some(false),
        }),
        audio: Some(AudioInput {
            device_present: true,
            channels: Some(2),
        }),
        input: Some(InputDevicesInput {
            gamepads: 1,
            wheels: Some(0),
        }),
        ..ProbeInputs::default()
    }
}
