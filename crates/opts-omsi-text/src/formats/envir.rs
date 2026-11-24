//! Parser for `envir.cfg`: sky textures, twilight range and the day/night light colour tables.
//!
//! Every colour table holds five states (sun at nadir, twilight start, sunrise, twilight end,
//! zenith) of three 0–255 channels, in that order.

use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

/// Number of sun states in a colour table.
pub const LIGHT_STATES: usize = 5;

/// Sky and light configuration of an OMSI installation.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvirConfig {
    /// Day, twilight and night sky textures (installation-relative, backslash separated).
    pub sky_textures: Vec<String>,
    /// Sun elevation (degrees) where twilight starts (negative) and ends (positive).
    pub twilight: (f32, f32),
    /// Direct sunlight colours per sun state.
    pub light_a: [[f32; 3]; LIGHT_STATES],
    /// Secondary (from above) light colours per sun state.
    pub light_b: [[f32; 3]; LIGHT_STATES],
    /// Ambient light colours per sun state.
    pub light_c: [[f32; 3]; LIGHT_STATES],
}

fn table(params: &[String]) -> Option<[[f32; 3]; LIGHT_STATES]> {
    let values: Vec<f32> = params
        .iter()
        .map_while(|p| parse_float(p).ok())
        .take(LIGHT_STATES * 3)
        .collect();
    if values.len() < LIGHT_STATES * 3 {
        return None;
    }
    let mut out = [[0.0; 3]; LIGHT_STATES];
    for (i, v) in values.iter().enumerate() {
        out[i / 3][i % 3] = *v / 255.0;
    }
    Some(out)
}

impl EnvirConfig {
    /// Parses the raw bytes of an `envir.cfg`.
    ///
    /// # Errors
    /// Returns a message naming the first missing or malformed section.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let blocks = scan_blocks(&decode_bytes(bytes));
        let find = |kw: &str| blocks.iter().find(|b| b.keyword == kw);
        let sky_textures: Vec<String> = find("sky_textures")
            .map(|b| {
                b.parameters
                    .iter()
                    .take(3)
                    .map(|p| p.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        if sky_textures.len() < 3 {
            return Err("[sky_textures] needs three texture paths".into());
        }
        let twilight = find("twilight_start_end")
            .and_then(|b| {
                Some((
                    parse_float(b.parameters.first()?).ok()?,
                    parse_float(b.parameters.get(1)?).ok()?,
                ))
            })
            .ok_or("[twilight_start_end] needs two numbers")?;
        let get = |kw: &str| {
            find(kw)
                .and_then(|b| table(&b.parameters))
                .ok_or_else(|| format!("[{kw}] needs {} numbers", LIGHT_STATES * 3))
        };
        Ok(Self {
            sky_textures,
            twilight,
            light_a: get("lightcolor_a")?,
            light_b: get("lightcolor_b")?,
            light_c: get("lightcolor_c")?,
        })
    }

    /// Interpolates a colour table at the given sun elevation (degrees above the horizon).
    #[must_use]
    pub fn light_at(&self, table: &[[f32; 3]; LIGHT_STATES], elevation_deg: f32) -> [f32; 3] {
        let knots = [-90.0, self.twilight.0, 0.0, self.twilight.1, 90.0];
        let e = elevation_deg.clamp(-90.0, 90.0);
        let i = (0..LIGHT_STATES - 1)
            .find(|&i| e <= knots[i + 1])
            .unwrap_or(LIGHT_STATES - 2);
        let span = (knots[i + 1] - knots[i]).max(f32::EPSILON);
        let t = ((e - knots[i]) / span).clamp(0.0, 1.0);
        std::array::from_fn(|c| table[i][c] + (table[i + 1][c] - table[i][c]) * t)
    }

    /// Blend weights of the (day, twilight, night) sky textures at the given sun elevation.
    #[must_use]
    pub fn sky_weights(&self, elevation_deg: f32) -> [f32; 3] {
        let (start, end) = self.twilight;
        if elevation_deg <= start {
            [0.0, 0.0, 1.0]
        } else if elevation_deg < 0.0 {
            let t = (elevation_deg - start) / (0.0 - start).max(f32::EPSILON);
            [0.0, t, 1.0 - t]
        } else if elevation_deg < end {
            let t = elevation_deg / end.max(f32::EPSILON);
            [t, 1.0 - t, 0.0]
        } else {
            [1.0, 0.0, 0.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        let mut s = String::from("intro text\n[sky_textures]\nTexture\\a.bmp\nTexture\\b.bmp\nTexture\\c.bmp\n[twilight_start_end]\n-18\n10\n");
        for (kw, base) in [("A", 0), ("B", 5), ("C", 10)] {
            s.push_str(&format!("[lightcolor_{kw}]\n"));
            for i in 0..15 {
                s.push_str(&format!("{}\n", (base + i) % 256));
            }
            s.push_str("Alt:\n99\n");
        }
        s
    }

    #[test]
    fn parses_sections() {
        let cfg = EnvirConfig::parse(sample().as_bytes()).unwrap();
        assert_eq!(cfg.sky_textures[2], "Texture\\c.bmp");
        assert_eq!(cfg.twilight, (-18.0, 10.0));
        assert!((cfg.light_a[1][0] - 3.0 / 255.0).abs() < 1e-6);
        assert!((cfg.light_b[0][0] - 5.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn interpolates_between_states() {
        let cfg = EnvirConfig::parse(sample().as_bytes()).unwrap();
        let at_sunrise = cfg.light_at(&cfg.light_a, 0.0);
        assert!((at_sunrise[0] - 6.0 / 255.0).abs() < 1e-6);
        let mid = cfg.light_at(&cfg.light_a, -9.0);
        assert!(mid[0] > cfg.light_a[1][0] && mid[0] < cfg.light_a[2][0]);
        assert_eq!(cfg.sky_weights(-30.0), [0.0, 0.0, 1.0]);
        assert_eq!(cfg.sky_weights(0.0), [0.0, 1.0, 0.0]);
        assert_eq!(cfg.sky_weights(45.0), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn rejects_missing_sections() {
        assert!(EnvirConfig::parse(b"[sky_textures]\na\nb\nc\n").is_err());
    }
}
