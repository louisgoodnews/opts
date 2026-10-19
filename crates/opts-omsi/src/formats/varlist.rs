//! Script variable name lists (`*_varlist.txt`, `*_stringvarlist.txt`): one name per line;
//! the position is the variable index (`docs/src/formats/text/vehicles/varnamelist.md`).

use super::text::decode_bytes;

/// Parsed variable list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VarList {
    /// Names in file order (blank lines skipped).
    pub names: Vec<String>,
}

impl VarList {
    /// Parses a variable list.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        Self {
            names: decode_bytes(bytes)
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_lines_are_skipped_and_names_trimmed() {
        let v = VarList::parse(b"lights_sw_blinker\r\n\r\n  lights_rueck \r\n");
        assert_eq!(v.names, ["lights_sw_blinker", "lights_rueck"]);
    }
}
