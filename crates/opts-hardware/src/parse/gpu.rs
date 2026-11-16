//! Classes of GPU device type, backend and DRM display mode.

use crate::profile::{GpuBackend, GpuClass};

/// Class of a device type name as the renderer reports it (`DiscreteGpu`, `integrated`, ...).
#[must_use]
pub fn gpu_class(s: &str) -> GpuClass {
    let l = s.to_ascii_lowercase();
    if l.contains("discrete") {
        GpuClass::Discrete
    } else if l.contains("integrated") {
        GpuClass::Integrated
    } else if l.contains("virtual") {
        GpuClass::Virtual
    } else if l.contains("cpu") || l.contains("software") {
        GpuClass::Software
    } else {
        GpuClass::Other
    }
}

/// Class of a backend name (`Vulkan`, `Dx12`, `Metal`, `Gl`).
#[must_use]
pub fn gpu_backend(s: &str) -> GpuBackend {
    let l = s.to_ascii_lowercase();
    if l.contains("vulkan") {
        GpuBackend::Vulkan
    } else if l.contains("dx12") || l.contains("d3d12") {
        GpuBackend::Dx12
    } else if l.contains("metal") {
        GpuBackend::Metal
    } else if l.contains("gl") {
        GpuBackend::Gl
    } else {
        GpuBackend::Other
    }
}

/// First `WIDTHxHEIGHT` line of a DRM `modes` file (the preferred mode).
#[must_use]
pub fn drm_first_mode(text: &str) -> Option<(u32, u32)> {
    let line = text.lines().next()?;
    let line = line.split(['i', 'p']).next().unwrap_or(line);
    let (w, h) = line.trim().split_once('x')?;
    let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}
