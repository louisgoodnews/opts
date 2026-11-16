//! DLSS availability.

use super::*;

/// Why DLSS is not available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlssNo {
    /// Not an NVIDIA adapter.
    NotNvidia,
    /// The OS is neither Windows nor Linux.
    OsUnsupported,
    /// The graphics API is not Vulkan or Direct3D 12.
    BackendUnsupported,
    /// No hardware ray tracing (no RTX tensor cores).
    NoRayTracing,
    /// The DLSS runtime cannot be loaded.
    NoRuntime,
}

impl DlssNo {
    /// Stable key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotNvidia => "not_nvidia",
            Self::OsUnsupported => "os_unsupported",
            Self::BackendUnsupported => "backend_unsupported",
            Self::NoRayTracing => "no_ray_tracing",
            Self::NoRuntime => "no_runtime",
        }
    }
}

/// Answer of [`dlss_available`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// The hardware can run DLSS (the runtime is present or not yet known).
    Yes,
    /// It cannot, and why.
    No(DlssNo),
    /// Not enough facts.
    Unknown,
}

/// Can this machine run DLSS ([M-0048](../../../docs/src/roadmap/phase-5/M-0048.md))? Needs an NVIDIA
/// adapter with ray tracing on Windows or Linux over Vulkan or Direct3D 12.
#[must_use]
pub fn dlss_available(p: &HardwareProfile) -> Availability {
    use Availability::{No, Unknown, Yes};
    if let Some(v) = p.gpu.vendor.get() {
        if v != GpuVendor::Nvidia {
            return No(DlssNo::NotNvidia);
        }
    }
    if let Some(os) = p.os.family.get() {
        if !matches!(os, OsFamily::Linux | OsFamily::Windows) {
            return No(DlssNo::OsUnsupported);
        }
    }
    if let Some(b) = p.gpu.backend.get() {
        if !matches!(b, GpuBackend::Vulkan | GpuBackend::Dx12) {
            return No(DlssNo::BackendUnsupported);
        }
    }
    if p.gpu.ray_tracing.get() == Some(false) {
        return No(DlssNo::NoRayTracing);
    }
    if p.gpu.dlss_runtime.get() == Some(false) {
        return No(DlssNo::NoRuntime);
    }
    if p.gpu.vendor.get().is_none() || p.gpu.ray_tracing.get().is_none() {
        return Unknown;
    }
    Yes
}
