//! Methods of the hardware profile: loading and saving.

use super::*;

impl HardwareProfile {
    /// Serialises to pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a profile; unknown fields are ignored, missing ones are `Unknown(not_probed)`,
    /// a schema newer than [`SCHEMA_VERSION`] is refused.
    ///
    /// # Errors
    /// [`LoadError`] when the text is not a profile or is too new.
    pub fn from_json(text: &str) -> Result<Self, LoadError> {
        let p: Self = serde_json::from_str(text).map_err(|e| LoadError::Parse(e.to_string()))?;
        if p.schema > SCHEMA_VERSION {
            return Err(LoadError::TooNew(p.schema));
        }
        Ok(p)
    }

    /// Hash of the coarse hardware *class* (FNV-1a over bucketed values). Many machines share a
    /// class; it is used only to decide whether cached benchmarks still apply.
    #[must_use]
    pub fn class_hash(&self) -> String {
        let bucket = |v: Option<u64>, base: u64| {
            v.map_or(0, |x| 64 - (x / base.max(1)).leading_zeros() as u64)
        };
        let parts = [
            format!("{:?}", self.os.family.get()),
            format!("{:?}", self.cpu.arch.get()),
            format!("{:?}", self.cpu.vendor.get()),
            bucket(self.cpu.logical_cores.get().map(u64::from), 1).to_string(),
            bucket(self.memory.total_mib.get(), 1024).to_string(),
            format!("{:?}", self.gpu.vendor.get()),
            format!("{:?}", self.gpu.device_class.get()),
            bucket(self.gpu.vram_mib.get(), 1024).to_string(),
            format!("{:?}", self.storage.class.get()),
        ];
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in parts.join("|").bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{h:016x}")
    }
}
