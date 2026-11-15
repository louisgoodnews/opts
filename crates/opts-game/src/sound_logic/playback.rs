//! What a definition does when it is active: its playback kind, from its keywords.

use super::*;

/// What a definition does when it is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    /// A loop while active.
    Loop,
    /// Once, when the conditions turn true.
    OnEdge,
    /// Once per raised event.
    OnTrigger,
    /// Uses a condition mode that is not understood.
    Unsupported,
}

/// Reads a definition's keywords into what it does.
#[must_use]
pub fn playback_of(d: &SoundDef) -> Playback {
    if d.conditions
        .iter()
        .any(|c| compare(c.mode, 0.0, 0.0).is_none())
    {
        return Playback::Unsupported;
    }
    if !d.triggers.is_empty() {
        return Playback::OnTrigger;
    }
    if !d.conditions.is_empty() && d.noloop {
        return Playback::OnEdge;
    }
    if d.kind == SoundKind::Loop || !d.noloop {
        return Playback::Loop;
    }
    // `[noloop]` alone: nothing starts it.
    Playback::Unsupported
}
