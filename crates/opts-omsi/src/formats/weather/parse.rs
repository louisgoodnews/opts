//! Parser of a `.owt` weather file: keyword blocks into a [`WeatherFile`].

use super::*;

impl WeatherFile {
    /// Parses a `.owt` (or a scenario's `.osn.owt`). Blocks that are missing or malformed keep
    /// the defaults of [`WeatherFile::default`]; a file without any known block is an error.
    ///
    /// # Errors
    /// Returns a message when the text holds none of the weather keywords.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let blocks = scan_blocks_with(&decode_bytes(bytes), arity);
        let mut w = Self::default();
        let mut known = 0;
        for b in &blocks {
            let p = &b.parameters;
            match b.keyword.as_str() {
                "name" => {
                    w.name = p.first().cloned().unwrap_or_default();
                }
                "description" => w.description = p.join("\n"),
                "fog" => {
                    if let Some((v, d)) = pair(b) {
                        w.fog = Fog {
                            visibility_m: v,
                            density: d,
                        };
                    }
                }
                "wind" => {
                    if let Some((d, s)) = pair(b) {
                        w.wind = Wind {
                            direction_deg: d,
                            speed: s,
                        };
                    }
                }
                "temp" => {
                    if let Some((t, d)) = pair(b) {
                        w.temp = Temperature {
                            celsius: t,
                            dew_point_c: d,
                        };
                    }
                }
                "press" => {
                    if let Some(v) = number(p, 0) {
                        w.pressure_hpa = v;
                    }
                }
                "clouds" => {
                    // A number as the type (`-1`) means no clouds.
                    let kind = p.first().map(|s| s.trim()).unwrap_or_default();
                    w.clouds =
                        (!kind.is_empty() && parse_float(kind).is_err()).then(|| CloudLayer {
                            kind: kind.to_owned(),
                            base_m: number(p, 1).unwrap_or(0.0),
                        });
                }
                "precip" => {
                    w.precip = Precip {
                        kind: PrecipKind::from_code(number(p, 0).unwrap_or(0.0)),
                        intensity: number(p, 1).unwrap_or(0.0),
                        extra: [
                            number(p, 2).unwrap_or(0.0),
                            number(p, 3).unwrap_or(0.0),
                            number(p, 4).unwrap_or(0.0),
                        ],
                    };
                }
                "groundwet" => {
                    for (i, v) in w.ground_wet.iter_mut().enumerate() {
                        *v = number(p, i).unwrap_or(0.0);
                    }
                }
                "snow" => w.snow = true,
                "snowonroad" => w.snow_on_road = true,
                "end" => {}
                _ => continue,
            }
            known += 1;
        }
        if known == 0 {
            return Err("no weather keyword found".into());
        }
        Ok(w)
    }

    /// The name without the angle brackets some files use.
    #[must_use]
    pub fn clean_name(&self) -> &str {
        strip_brackets(&self.name)
    }
}
