//! Counts and sizes read from device and kernel text files (caches, joysticks, sound cards).

/// `32K`, `1024K`, `32M`, `32768K\n` to KiB.
#[must_use]
pub fn cache_kib(s: &str) -> Option<u32> {
    let t = s.trim();
    let (num, mult) = if let Some(n) = t.strip_suffix(['K', 'k']) {
        (n, 1)
    } else if let Some(n) = t.strip_suffix(['M', 'm']) {
        (n, 1024)
    } else {
        (t, 1)
    };
    num.trim().parse::<u32>().ok()?.checked_mul(mult)
}

/// Number of gamepad or joystick handlers (`js0`, `js1`) in `/proc/bus/input/devices`; only the
/// `Handlers=` lines are inspected.
#[must_use]
pub fn count_joysticks(text: &str) -> u32 {
    let n = text
        .lines()
        .filter(|l| l.starts_with("H: Handlers="))
        .filter(|l| l.split_whitespace().any(is_js_handler))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn is_js_handler(w: &str) -> bool {
    w.strip_prefix("js")
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Number of sound cards in `/proc/asound/cards` (`--- no soundcards ---` gives 0).
#[must_use]
pub fn count_sound_cards(text: &str) -> u32 {
    let n = text
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.split_whitespace()
                .next()
                .is_some_and(|w| w.bytes().all(|b| b.is_ascii_digit()))
                && t.contains('[')
        })
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Bytes to whole MiB (rounded down).
#[must_use]
pub const fn mib(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}
