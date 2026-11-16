//! Per-material and per-keyword classification of the text files (the census scan).

use super::*;

/// The `[matl]` block being read.
pub(super) struct Mat {
    in_mat: bool,
    tex: String,
    has_env: bool,
    alpha: String,
}

impl Mat {
    pub(super) fn new() -> Self {
        Self {
            in_mat: false,
            tex: String::new(),
            has_env: false,
            alpha: String::from("-"),
        }
    }
}

impl Census {
    /// Classifies the finished material by its texture name.
    pub(super) fn flush(&mut self, m: &Mat) {
        if !m.in_mat {
            return;
        }
        let t = m.tex.rsplit(['/', '\\']).next().unwrap_or(&m.tex);
        let g = has_any(t, &GLASS_WORDS);
        let mir = has_any(t, &MIRROR_WORDS);
        if g {
            self.glass_mats += 1;
            *self.glass_alpha.entry(m.alpha.to_string()).or_default() += 1;
            *self.glass_tex.entry(t.to_string()).or_default() += 1;
            if m.has_env {
                self.glass_env += 1;
            }
        }
        if mir {
            self.mirror_mats += 1;
            *self.mirror_tex.entry(t.to_string()).or_default() += 1;
            if m.has_env {
                self.mirror_env += 1;
            }
        }
        if m.has_env && !g && !mir {
            self.env_nonglass_mats += 1;
        }
    }

    /// File-name statistics (any file type); `fname` is the lower-case base name.
    pub(super) fn scan_name(&mut self, rel: &str, ext: &str, fname: &str) {
        if has_any(fname, &MIRROR_WORDS) {
            let key = if matches!(ext, "o3d" | "x" | "bmp" | "dds" | "tga" | "jpg" | "png") {
                "mirror-named file"
            } else {
                "mirror-named other"
            };
            self.name_hits.entry(key).or_default().push(rel.to_string());
        }
        if fname.contains("envmap") || fname.contains("env_map") || fname.contains("reflect") {
            *self.env_files_bmp.entry(fname.to_string()).or_default() += 1;
        }
    }

    /// One `[keyword]` line (`lines[i]`) of a text file; `at` is (relative path, unit).
    pub(super) fn on_header(&mut self, m: &mut Mat, lines: &[String], i: usize, at: (&str, &str)) {
        let (rel, unit) = at;
        let l = &lines[i];
        let name = &l[1..l.len() - 1];
        let next = lines.get(i + 1).map(String::as_str).unwrap_or("");
        if name == "matl" {
            self.flush(m);
            *m = Mat {
                in_mat: true,
                tex: next.to_string(),
                has_env: false,
                alpha: "-".into(),
            };
            self.total_matl += 1;
            self.total_matl_files.insert(rel.to_string());
        }
        let interesting = matches!(
            name,
            "matl_envmap"
                | "matl_envmap_mask"
                | "matl_alpha"
                | "matl_transmap"
                | "matl_bumpmap"
                | "matl_nightmap"
                | "matl_lightmap"
                | "matl_noZwrite"
                | "matl_nozwrite"
                | "matl_nozcheck"
        ) || has_any(name, &["mirror", "spiegel", "reflect", "cube", "env"]);
        if interesting {
            self.kw.entry(name.to_string()).or_default().add(rel, unit);
        }
        if name == "matl_alpha" {
            *self.alpha_vals.entry(next.to_string()).or_default() += 1;
            m.alpha = next.to_string();
        }
        if name == "matl_envmap" {
            m.has_env = true;
            *self.envmap_tex.entry(next.to_string()).or_default() += 1;
            let fac = lines.get(i + 2).map(String::as_str).unwrap_or("");
            *self.envmap_factor.entry(fac.to_string()).or_default() += 1;
        }
    }
}
