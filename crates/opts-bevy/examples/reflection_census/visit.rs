//! Walks one file: its unit, the name statistics and the lines of text files.

use super::scan::Mat;
use super::*;

impl Census {
    /// Scans the trimmed, lower-cased lines of one text file.
    fn scan_lines(&mut self, lines: &[String], ext: &str, at: (&str, &str)) {
        let mut m = Mat::new();
        for (i, l) in lines.iter().enumerate() {
            if l.starts_with('[') && l.ends_with(']') {
                self.on_header(&mut m, lines, i, at);
            } else if ext == "osc" || ext == "txt" || ext == "cfg" {
                // Script or free text mentioning mirrors.
                if (l.contains("mirror") || l.contains("spiegel"))
                    && !l.starts_with("//")
                    && !l.starts_with('#')
                {
                    self.name_hits
                        .entry("text line mentions mirror")
                        .or_default()
                        .push(format!(
                            "{}: {}",
                            at.0,
                            l.chars().take(80).collect::<String>()
                        ));
                }
            }
        }
        self.flush(&m);
    }

    pub(super) fn visit(&mut self, f: &Path, root: &Path) {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let unit = unit_of(&rel);
        if unit.starts_with("V:") {
            self.vehicles.insert(unit.clone());
        }
        if unit.starts_with("O:") {
            self.objects.insert(unit.clone());
        }
        if ext == "sco" {
            self.sco_files += 1;
        }
        let lower_rel = rel.to_lowercase();
        self.scan_name(&rel, &ext, lower_rel.rsplit('/').next().unwrap_or(""));
        if !TEXT_EXT.contains(&ext.as_str()) {
            return;
        }
        let Ok(bytes) = std::fs::read(f) else {
            return;
        };
        self.text_files += 1;
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let lines: Vec<String> = text.lines().map(|l| l.trim().to_lowercase()).collect();
        self.scan_lines(&lines, &ext, (&rel, &unit));
    }
}
