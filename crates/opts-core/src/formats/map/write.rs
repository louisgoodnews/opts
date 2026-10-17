//! Writers for map text files: `global.cfg` ([`MapManifest::to_text`]) and the `[object]`
//! records of a tile ([`MapTile::to_text`]).
//!
//! The output is what the parsers in this module read back: `parse(write(x)) == x` for every
//! value the writers support. OMSI's text scanner drops blank lines and comment lines and trims
//! every line, so a value that is empty, has surrounding white space, spans several lines or
//! looks like a keyword or a comment cannot be written; the writers reject it with
//! [`MapWriteError`] instead of producing a file that parses differently.
//!
//! See `docs/src/formats/map/global-cfg.md` and `docs/src/formats/map/tiles.md`.

use std::fmt::Write as _;

use super::{MapManifest, MapObjectInstance, MapTile};

/// Why a value cannot be written as one line of an OMSI text file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MapWriteError {
    /// The value would not survive the text scanner (empty, multi-line, padded, keyword or comment).
    #[error("{field}: value {value:?} cannot be written as one OMSI text line ({reason})")]
    UnwritableLine {
        /// Which field the value belongs to.
        field: &'static str,
        /// The offending value.
        value: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// The tile has records the writer does not support (splines, rules, attachments, flags).
    #[error("tile writer supports only [version] and [object] records; found {0}")]
    UnsupportedTileContent(&'static str),
}

/// Checks that `value` reads back unchanged as one parameter line.
fn line(field: &'static str, value: &str) -> Result<String, MapWriteError> {
    let reject = |reason| {
        Err(MapWriteError::UnwritableLine {
            field,
            value: value.to_string(),
            reason,
        })
    };
    if value.is_empty() {
        return reject("empty lines are dropped by the scanner");
    }
    if value.contains(['\n', '\r']) {
        return reject("contains a line break");
    }
    if value.trim() != value {
        return reject("leading or trailing white space is trimmed");
    }
    if value.starts_with("//") || value.starts_with('#') {
        return reject("comment lines are dropped");
    }
    if value.starts_with('[') && value.ends_with(']') {
        return reject("reads as a keyword");
    }
    Ok(value.to_string())
}

/// Checks that a string value reads back unchanged. The tile reader takes string values line by
/// line, so blank and comment-like values survive; padding, line breaks and keyword lines do not.
fn string_value(field: &'static str, value: &str) -> Result<String, MapWriteError> {
    if value.is_empty() || value.starts_with("//") || value.starts_with('#') {
        if value.contains(['\n', '\r']) {
            return line(field, value);
        }
        if value.trim() != value {
            return line(field, value);
        }
        return Ok(value.to_string());
    }
    line(field, value)
}

/// Appends a keyword followed by its parameter lines.
fn block(out: &mut String, keyword: &str, params: &[String]) {
    let _ = writeln!(out, "[{keyword}]");
    for p in params {
        let _ = writeln!(out, "{p}");
    }
    out.push('\n');
}

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

impl MapTile {
    /// Serialises the tile's `[version]` and `[object]` records as `tile_X_Y.map` text.
    ///
    /// Version 10 tiles are written without pitch and roll, as that version stores them. In
    /// other versions a missing pitch or roll is written as `0`, so it reads back as `Some(0.0)`.
    ///
    /// # Errors
    /// [`MapWriteError::UnsupportedTileContent`] when the tile has splines, rules, spline
    /// attachments or terrain/water flags; [`MapWriteError::UnwritableLine`] for a string
    /// (path or string variable) that would not read back unchanged.
    pub fn to_text(&self) -> Result<String, MapWriteError> {
        let unsupported = [
            (!self.splines.is_empty(), "[spline]"),
            (!self.rules.is_empty(), "[rule]"),
            (!self.spline_attachments.is_empty(), "[splineAttachement]"),
            (self.has_terrain, "[terrain]"),
            (self.has_water, "[water]"),
            (self.has_variable_terrain, "[variable_terrain]"),
            (
                self.has_variable_terrain_lightmap,
                "[variable_terrainlightmap]",
            ),
        ];
        if let Some((_, what)) = unsupported.iter().find(|(present, _)| *present) {
            return Err(MapWriteError::UnsupportedTileContent(what));
        }
        let mut out = String::new();
        if let Some(v) = self.version {
            block(&mut out, "version", &[v.to_string()]);
        }
        let legacy = self.version == Some(10);
        for obj in &self.objects {
            block(&mut out, "object", &object_lines(obj, legacy)?);
        }
        Ok(out)
    }
}

/// The parameter lines of one `[object]` record.
fn object_lines(obj: &MapObjectInstance, legacy: bool) -> Result<Vec<String>, MapWriteError> {
    let mut p = vec![
        obj.flag.to_string(),
        line("object sco", &obj.sco_file)?,
        obj.id_code.to_string(),
    ];
    p.extend(
        [
            obj.position.x,
            obj.position.y,
            obj.position.z,
            obj.heading_deg,
        ]
        .map(|v| v.to_string()),
    );
    if !legacy {
        p.push(obj.pitch_deg.unwrap_or(0.0).to_string());
        p.push(obj.roll_deg.unwrap_or(0.0).to_string());
    }
    p.push(obj.string_vars.len().to_string());
    for s in &obj.string_vars {
        p.push(string_value("object string variable", s)?);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::formats::map::{MapEntryPoint, MapGroundTex, MapSeason, MapTileEntry};

    fn manifest() -> MapManifest {
        MapManifest {
            name: "OPTS Scenery Benchmark".into(),
            friendly_name: Some("Benchmark".into()),
            description: vec!["Generated map.".into(), "Second line".into()],
            version: Some(14),
            next_id_code: Some(3),
            money_system: Some(r"Money\DM\DM.cfg".into()),
            ticket_pack: None,
            standard_depot: Some("Depot".into()),
            years: Some((1990, 2010)),
            real_year_offset: Some(-3),
            repair_time_min: Some(2.5),
            world_coordinates: true,
            real_rail: false,
            dyn_helper_active: true,
            tiles: vec![MapTileEntry {
                x: 0,
                y: -1,
                file_name: "tile_0_-1.map".into(),
            }],
            entry_points: vec![MapEntryPoint {
                id: 0,
                id_code: 1,
                position: Vec3::new(20.0, 0.0, 20.0),
                direction: Vec3::new(0.0, 0.25, 1.0),
                name: "Benchmark".into(),
            }],
            ground_textures: vec![MapGroundTex {
                ground_texture: r"texture\grass.dds".into(),
                detail_texture: r"texture\detail.dds".into(),
                params: [1.0, 0.5, 30.0],
            }],
            seasons: vec![MapSeason {
                season_id: 3,
                start_day: 330,
                end_day: 60,
            }],
            traffic_density_road: vec![(0.0, 0.1), (8.5, 1.0)],
            traffic_density_passenger: vec![(12.0, 0.75)],
        }
    }

    fn object(strings: &[&str]) -> MapObjectInstance {
        MapObjectInstance {
            flag: 0,
            sco_file: r"Sceneryobjects\OPTS_Test\box.sco".into(),
            id_code: 2,
            position: Vec3::new(150.0, 150.25, -0.5),
            heading_deg: 37.0,
            pitch_deg: Some(1.5),
            roll_deg: Some(-2.0),
            string_vars: strings.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn manifest_round_trips() {
        let m = manifest();
        let text = m.to_text().unwrap();
        assert_eq!(MapManifest::from_text(&text).unwrap(), m);
    }

    #[test]
    fn minimal_manifest_round_trips() {
        let m = MapManifest {
            name: "x".into(),
            ..MapManifest::default()
        };
        assert_eq!(MapManifest::from_text(&m.to_text().unwrap()).unwrap(), m);
    }

    #[test]
    fn tile_objects_round_trip() {
        let tile = MapTile {
            version: Some(14),
            objects: vec![object(&["OPTS-A1", "OPTS A2 with spaces"]), object(&[])],
            ..MapTile::default()
        };
        let text = tile.to_text().unwrap();
        assert_eq!(MapTile::from_text(&text).unwrap(), tile);
    }

    /// Blank and comment-like string values are read line by line, so they round-trip.
    #[test]
    fn blank_and_comment_like_strings_round_trip() {
        let tile = MapTile {
            version: Some(14),
            objects: vec![object(&["", "// not a comment", "#1"]), object(&["x"])],
            ..MapTile::default()
        };
        let text = tile.to_text().unwrap();
        assert_eq!(MapTile::from_text(&text).unwrap(), tile);
    }

    #[test]
    fn legacy_tile_round_trips_without_pitch() {
        let mut obj = object(&["a"]);
        obj.pitch_deg = None;
        obj.roll_deg = None;
        let tile = MapTile {
            version: Some(10),
            objects: vec![obj],
            ..MapTile::default()
        };
        assert_eq!(MapTile::from_text(&tile.to_text().unwrap()).unwrap(), tile);
    }

    #[test]
    fn unwritable_strings_are_rejected() {
        for bad in [" padded", "two\nlines", "[keyword]", "# padded "] {
            let tile = MapTile {
                version: Some(14),
                objects: vec![object(&[bad])],
                ..MapTile::default()
            };
            assert!(
                matches!(tile.to_text(), Err(MapWriteError::UnwritableLine { .. })),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn splines_are_not_written() {
        let tile = MapTile {
            has_terrain: true,
            ..MapTile::default()
        };
        assert!(matches!(
            tile.to_text(),
            Err(MapWriteError::UnsupportedTileContent(_))
        ));
    }
}
