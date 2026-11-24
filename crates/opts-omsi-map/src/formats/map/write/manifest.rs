//! Writer of `MapManifest`: the map header and the element lists of a manifest file.

use super::*;

impl MapManifest {
    /// Serialises the manifest as `global.cfg` text (UTF-8, `\n` line ends).
    ///
    /// Every field of [`MapManifest`] is written. Entry-point records get the unparsed lines
    /// (2, 6 and 10 of each 12-line record) as `0`, `0` and `18`, the values OMSI writes.
    ///
    /// # Errors
    /// [`MapWriteError::UnwritableLine`] for a string that would not read back unchanged.
    pub fn to_text(&self) -> Result<String, MapWriteError> {
        let mut out = String::new();
        block(&mut out, "name", &[line("name", &self.name)?]);
        if let Some(f) = &self.friendly_name {
            block(&mut out, "friendlyname", &[line("friendlyname", f)?]);
        }
        if !self.description.is_empty() {
            let lines = self
                .description
                .iter()
                .map(|d| line("description", d))
                .collect::<Result<Vec<_>, _>>()?;
            block(&mut out, "description", &lines);
            block(&mut out, "end", &[]);
        }
        let optional_numbers = [
            ("version", self.version.map(|v| v.to_string())),
            ("NextIDCode", self.next_id_code.map(|v| v.to_string())),
            (
                "realyearoffset",
                self.real_year_offset.map(|v| v.to_string()),
            ),
            (
                "repair_time_min",
                self.repair_time_min.map(|v| v.to_string()),
            ),
        ];
        for (kw, value) in optional_numbers {
            if let Some(v) = value {
                block(&mut out, kw, &[v]);
            }
        }
        let optional_strings = [
            ("moneysystem", &self.money_system),
            ("ticketpack", &self.ticket_pack),
            ("standarddepot", &self.standard_depot),
        ];
        for (kw, value) in optional_strings {
            if let Some(v) = value {
                block(&mut out, kw, &[line(kw, v)?]);
            }
        }
        if let Some((a, b)) = self.years {
            block(&mut out, "years", &[a.to_string(), b.to_string()]);
        }
        for (flag, kw) in [
            (self.world_coordinates, "worldcoordinates"),
            (self.real_rail, "realrail"),
            (self.dyn_helper_active, "dynhelperactive"),
        ] {
            if flag {
                block(&mut out, kw, &[]);
            }
        }
        self.write_lists(&mut out)?;
        Ok(out)
    }

    /// Ground textures, seasons, traffic curves, tiles and entry points.
    fn write_lists(&self, out: &mut String) -> Result<(), MapWriteError> {
        for g in &self.ground_textures {
            let mut p = vec![
                line("groundtex", &g.ground_texture)?,
                line("groundtex detail", &g.detail_texture)?,
            ];
            p.extend(g.params.iter().map(f32::to_string));
            block(out, "groundtex", &p);
        }
        for s in &self.seasons {
            let p = [s.season_id, s.start_day, s.end_day].map(|v| v.to_string());
            block(out, "addseason", &p);
        }
        for (kw, curve) in [
            ("trafficdensity_road", &self.traffic_density_road),
            ("trafficdensity_passenger", &self.traffic_density_passenger),
        ] {
            for (h, f) in curve {
                block(out, kw, &[h.to_string(), f.to_string()]);
            }
        }
        for t in &self.tiles {
            let p = [t.x.to_string(), t.y.to_string(), line("map", &t.file_name)?];
            block(out, "map", &p);
        }
        if !self.entry_points.is_empty() {
            let mut p = vec![self.entry_points.len().to_string()];
            for e in &self.entry_points {
                p.extend([e.id.to_string(), e.id_code.to_string(), "0".into()]);
                p.extend([e.position.x, e.position.y, e.position.z].map(|v| v.to_string()));
                p.push("0".into());
                p.extend([e.direction.x, e.direction.y, e.direction.z].map(|v| v.to_string()));
                p.push("18".into());
                p.push(line("entrypoint name", &e.name)?);
            }
            block(out, "entrypoints", &p);
        }
        Ok(())
    }
}
