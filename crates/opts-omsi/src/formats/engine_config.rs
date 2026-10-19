//! User input configuration: `Inputs/keyboard.cfg` (`docs/src/formats/text/engine-config.md`).
//!
//! `keyboard.cfg` has two section markers, `[game]` (engine commands) and `[vehicles]` (script
//! trigger names), each followed by `[entry]` blocks of three lines: action name, DirectInput
//! scan code and modifier mask. Only `keyboard.cfg` is parsed so far; `gamectrler.cfg` and
//! `options.cfg` are not.

use super::text::{decode_bytes, scan_blocks};

/// Section a key binding is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySection {
    /// `[game]`: engine commands (`sim_pause`, `view_set_driver`, …).
    Game,
    /// `[vehicles]`: names of vehicle script triggers (`bus_doorfront0`, `horn`, …).
    Vehicles,
}

/// One `[entry]` of `keyboard.cfg`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    /// Section the entry belongs to.
    pub section: KeySection,
    /// Action (for `[vehicles]`, a script trigger name).
    pub action: String,
    /// DirectInput keyboard scan code (index into the `.kyb` key-name tables).
    pub scan_code: u16,
    /// Modifier mask as written: bit 1 = held key, bit 2 = Shift, bit 4 = Ctrl.
    pub modifiers: u8,
}

impl KeyBinding {
    /// Bit 1: a held key. Only continuous controls carry it (`throttle`, `brake`,
    /// `steering_left`, `horn`, `blinker_left_set`, `cp_microphone`, `bus_rollband_up` but not
    /// `bus_rollband_up_step`), so it marks bindings that also fire `<action>_off` on release ❓.
    #[must_use]
    pub const fn held(&self) -> bool {
        self.modifiers & 1 != 0
    }

    /// Bit 2: Shift must be pressed.
    #[must_use]
    pub const fn shift(&self) -> bool {
        self.modifiers & 2 != 0
    }

    /// Bit 4: Ctrl must be pressed.
    #[must_use]
    pub const fn ctrl(&self) -> bool {
        self.modifiers & 4 != 0
    }
}

/// Parsed `keyboard.cfg`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyboardConfig {
    /// Entries in file order.
    pub bindings: Vec<KeyBinding>,
}

impl KeyboardConfig {
    /// Parses `keyboard.cfg`; entries before a section marker or with unreadable numbers are
    /// skipped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        let mut section = None;
        for block in scan_blocks(&decode_bytes(bytes)) {
            match (block.keyword.as_str(), block.parameters.as_slice()) {
                ("game", _) => section = Some(KeySection::Game),
                ("vehicles", _) => section = Some(KeySection::Vehicles),
                ("entry", [action, code, mods, ..]) => {
                    let (Some(section), Ok(scan_code), Ok(modifiers)) =
                        (section, code.trim().parse(), mods.trim().parse())
                    else {
                        continue;
                    };
                    out.bindings.push(KeyBinding {
                        section,
                        action: action.trim().to_string(),
                        scan_code,
                        modifiers,
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// The `[vehicles]` entries (script triggers).
    pub fn vehicle_bindings(&self) -> impl Iterator<Item = &KeyBinding> {
        self.bindings
            .iter()
            .filter(|b| b.section == KeySection::Vehicles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b"\r\n[game]\r\n\r\n[entry]\r\nsim_pause\r\n25\r\n0\r\n\r\n[vehicles]\r\n\r\n[entry]\r\nhorn\r\n35\r\n1\r\n\r\n[entry]\r\nchange_give\r\n20\r\n4\r\n\r\n[entry]\r\nbroken\r\nx\r\n0\r\n";

    #[test]
    fn entries_carry_their_section() {
        let cfg = KeyboardConfig::parse(SAMPLE);
        assert_eq!(cfg.bindings.len(), 3);
        assert_eq!(cfg.bindings[0].section, KeySection::Game);
        assert_eq!(cfg.bindings[0].action, "sim_pause");
        let vehicles: Vec<_> = cfg.vehicle_bindings().map(|b| b.action.as_str()).collect();
        assert_eq!(vehicles, ["horn", "change_give"]);
    }

    #[test]
    fn modifier_bits() {
        let cfg = KeyboardConfig::parse(SAMPLE);
        let horn = &cfg.bindings[1];
        assert_eq!(horn.scan_code, 35);
        assert!(horn.held() && !horn.shift() && !horn.ctrl());
        let change = &cfg.bindings[2];
        assert!(change.ctrl() && !change.held() && !change.shift());
    }

    #[test]
    fn entry_before_a_section_is_skipped() {
        assert!(KeyboardConfig::parse(b"[entry]\nhorn\n35\n1\n")
            .bindings
            .is_empty());
    }
}
