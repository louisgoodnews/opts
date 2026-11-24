//! Vehicle constant files (`*_constfile.txt`): `[const]` scalars and `[newcurve]` curves.
//!
//! Every line that is not a keyword (`[…]` alone on the line) is a comment, including prose
//! between entries (`docs/src/formats/text/vehicles/const.md`).

use super::text::decode_bytes;

/// A named piecewise-linear curve.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstCurve {
    /// Curve name.
    pub name: String,
    /// `(x, y)` points in file order (sorted ascending by the parser).
    pub points: Vec<(f32, f32)>,
}

/// Parsed constant file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConstFile {
    /// Scalars in file order (duplicates are kept; later entries win when applied).
    pub scalars: Vec<(String, f32)>,
    /// Curves in file order.
    pub curves: Vec<ConstCurve>,
    /// Human-readable warnings (skipped entries, unsorted curves).
    pub warnings: Vec<String>,
}

fn is_keyword(line: &str) -> Option<String> {
    let t = line.trim();
    (t.len() > 2 && t.starts_with('[') && t.ends_with(']'))
        .then(|| t[1..t.len() - 1].to_lowercase())
}

impl ConstFile {
    /// Parses a constant file; malformed entries are skipped with a warning.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let text = decode_bytes(bytes);
        let lines: Vec<&str> = text.lines().collect();
        let mut out = Self::default();
        let mut i = 0;
        while i < lines.len() {
            match is_keyword(lines[i]).as_deref() {
                Some("const") => i = out.scalar(&lines, i),
                Some("newcurve") => i = out.curve(&lines, i),
                Some("pnt") => {
                    out.warnings
                        .push(format!("line {}: [pnt] outside [newcurve] ignored", i + 1));
                    i += 1;
                }
                _ => i += 1,
            }
        }
        out
    }

    fn scalar(&mut self, lines: &[&str], at: usize) -> usize {
        let name = lines.get(at + 1).map(|l| l.trim());
        let value = lines.get(at + 2).and_then(|l| l.trim().parse::<f32>().ok());
        match (name, value) {
            (Some(n), Some(v)) if !n.is_empty() && is_keyword(n).is_none() => {
                if self.scalars.iter().any(|(k, _)| k.eq_ignore_ascii_case(n)) {
                    self.warnings.push(format!(
                        "line {}: duplicate constant `{n}`; last wins",
                        at + 1
                    ));
                }
                self.scalars.push((n.to_owned(), v));
                at + 3
            }
            _ => {
                self.warnings.push(format!(
                    "line {}: [const] without a numeric value skipped",
                    at + 1
                ));
                at + 1
            }
        }
    }

    fn curve(&mut self, lines: &[&str], at: usize) -> usize {
        let Some(name) = lines.get(at + 1).map(|l| l.trim().to_owned()) else {
            self.warnings.push(format!(
                "line {}: [newcurve] without a name skipped",
                at + 1
            ));
            return at + 1;
        };
        let mut points = Vec::new();
        let mut i = at + 2;
        while i < lines.len() {
            match is_keyword(lines[i]).as_deref() {
                Some("pnt") => {
                    let x = lines.get(i + 1).and_then(|l| l.trim().parse::<f32>().ok());
                    let y = lines.get(i + 2).and_then(|l| l.trim().parse::<f32>().ok());
                    if let (Some(x), Some(y)) = (x, y) {
                        points.push((x, y));
                        i += 3;
                    } else {
                        self.warnings
                            .push(format!("line {}: truncated [pnt] discarded", i + 1));
                        i += 1;
                    }
                }
                Some("const" | "newcurve") => break,
                _ => i += 1,
            }
        }
        if points.windows(2).any(|w| w[1].0 < w[0].0) {
            self.warnings
                .push(format!("curve `{name}`: points not ascending; sorted"));
            points.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        if points.is_empty() {
            self.warnings.push(format!("curve `{name}` has no points"));
        }
        self.curves.push(ConstCurve { name, points });
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_cst_001_to_003_scalars_comments_and_curves() {
        let f = ConstFile::parse(
            b"Abblendlicht: 35W\r\n[const]\r\nfoo\r\n0.35\r\n.....\r\n[newcurve]\r\nretarder\r\n[pnt]\r\n0\r\n0\r\n[pnt]\r\n10\r\n194\r\n[pnt]\r\n20\r\n388\r\n",
        );
        assert_eq!(f.scalars, [("foo".to_owned(), 0.35)]);
        assert_eq!(
            f.curves[0].points,
            [(0.0, 0.0), (10.0, 194.0), (20.0, 388.0)]
        );
        assert!(f.warnings.is_empty());
    }

    #[test]
    fn t_cst_005_and_006_duplicates_and_truncation_warn() {
        let f = ConstFile::parse(b"[const]\na\n1\n[const]\na\n2\n[newcurve]\nc\n[pnt]\n5\n");
        assert_eq!(f.scalars.len(), 2);
        assert!(f.warnings.iter().any(|w| w.contains("duplicate")));
        assert!(f.warnings.iter().any(|w| w.contains("truncated")));
        assert!(f.curves[0].points.is_empty());
    }

    #[test]
    fn missing_value_and_stray_pnt_are_skipped() {
        let f = ConstFile::parse(b"[pnt]\n1\n2\n[const]\nname\nabc\n");
        assert!(f.scalars.is_empty());
        assert_eq!(f.warnings.len(), 2);
    }
}
