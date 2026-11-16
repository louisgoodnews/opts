//! Identity of a profile: the map key, the slug, the Chrono id and the settings hash.

use super::*;

/// Identity of a profile: the map and the Chrono entry its date resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapKey {
    /// The map folder name, made filesystem-safe.
    pub map_id: String,
    /// `default` or the Chrono entry (its path below `Chrono/`), made filesystem-safe.
    pub chrono_id: String,
}

impl MapKey {
    /// The key of a map folder name and the Chrono entries active on the session date
    /// (`rel` paths below `Chrono/`; empty when none is active).
    #[must_use]
    pub fn new(map_folder: &str, active_chrono: &[&str]) -> Self {
        Self {
            map_id: slug(map_folder),
            chrono_id: chrono_id(active_chrono),
        }
    }

    /// The same map with no Chrono entry: the fallback of every Chrono key.
    #[must_use]
    pub fn map_default(&self) -> Self {
        Self {
            map_id: self.map_id.clone(),
            chrono_id: DEFAULT_CHRONO.to_string(),
        }
    }
}

/// Makes a name safe for a file name: letters, digits and `._- ` stay, everything else becomes
/// `%XX` per byte (so two different names never collide and the name stays readable).
#[must_use]
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for b in name.trim().bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b' ') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    // A leading dot would hide the folder, and "." / ".." are not names.
    if out.is_empty() || out.starts_with('.') {
        out.insert(0, '_');
    }
    out
}

/// The Chrono part of a key: `default` without an active entry, else the entries' paths sorted
/// and joined (nesting becomes `__`), cut and hashed when very long.
#[must_use]
pub fn chrono_id(active: &[&str]) -> String {
    if active.is_empty() {
        return DEFAULT_CHRONO.to_string();
    }
    let mut parts: Vec<String> = active.iter().map(|r| slug(&r.replace('/', "__"))).collect();
    parts.sort();
    let joined = parts.join("+");
    if joined.len() <= 80 {
        return joined;
    }
    format!("{}~{:08x}", &joined[..60], fnv32(joined.as_bytes()))
}

fn fnv32(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in bytes {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// A stable hash of the map-eligible values (the `settings_hash` of the metrics).
#[must_use]
pub fn settings_hash(values: &BTreeMap<String, String>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (k, v) in values {
        for b in k.bytes().chain(*b"=").chain(v.bytes()).chain(*b";") {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}
