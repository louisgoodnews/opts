//! Parser and data structures for OMSI map scenario manifests (`global.cfg`).
//!
//! `global.cfg` is the top-level manifest defining a map in OMSI. It declares
//! the map name, tile list, ground textures, seasons, traffic density curves,
//! money system bindings, and player spawn entry points.
//!
//! See specification in `docs/src/formats/map/global-cfg.md`.

use glam::Vec3;

use super::error::MapParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

/// Ground texture surface configuration (`[groundtex]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapGroundTex {
    /// Base ground texture bitmap filename.
    pub ground_texture: String,
    /// Detail texture bitmap filename.
    pub detail_texture: String,
    /// Numeric surface/scale parameters.
    pub params: [f32; 3],
}

/// Calendar season date mapping (`[addseason]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSeason {
    /// Season ID (1 = Spring, 2 = Autumn, 3 = Winter, 4 = Winter Snow).
    pub season_id: u32,
    /// Starting day of year (0..370).
    pub start_day: u32,
    /// Ending day of year (0..370).
    pub end_day: u32,
}

/// Player/AI vehicle entry spawn point (`[entrypoints]`).
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntryPoint {
    /// Entrypoint numeric identifier.
    pub id: u32,
    /// Object ID code on the tile.
    pub id_code: u32,
    /// Tile-local position in meters.
    pub position: Vec3,
    /// Normalized orientation direction vector.
    pub direction: Vec3,
    /// Friendly display name.
    pub name: String,
}

/// Reference to a map tile file (`[map]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTileEntry {
    /// Tile grid coordinate X (easting cell).
    pub x: i32,
    /// Tile grid coordinate Y (northing cell).
    pub y: i32,
    /// Tile filename (`tile_<x>_<y>.map`).
    pub file_name: String,
}

/// Complete representation of an OMSI map scenario manifest (`global.cfg`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapManifest {
    /// Internal map identifier / folder name.
    pub name: String,
    /// Human-readable friendly name.
    pub friendly_name: Option<String>,
    /// Multi-line description text lines.
    pub description: Vec<String>,
    /// Map version number.
    pub version: Option<u32>,
    /// Next available object ID code.
    pub next_id_code: Option<u32>,
    /// Path to money system configuration (`Money/<pack>/<pack>.cfg`).
    pub money_system: Option<String>,
    /// Path to ticket pack configuration.
    pub ticket_pack: Option<String>,
    /// Default vehicle depot name.
    pub standard_depot: Option<String>,
    /// Valid calendar year range: `(first_year, last_year)`.
    pub years: Option<(i32, i32)>,
    /// Calendar year offset.
    pub real_year_offset: Option<i32>,
    /// Average vehicle maintenance/repair time in minutes.
    pub repair_time_min: Option<f32>,
    /// Whether world GPS coordinates are enabled.
    pub world_coordinates: bool,
    /// Whether realistic railway signaling rules apply.
    pub real_rail: bool,
    /// Whether dynamic helper arrows are active.
    pub dyn_helper_active: bool,
    /// Grid tiles comprising this map.
    pub tiles: Vec<MapTileEntry>,
    /// Vehicle entry and spawn points.
    pub entry_points: Vec<MapEntryPoint>,
    /// Ground texture palette mappings.
    pub ground_textures: Vec<MapGroundTex>,
    /// Calendar season definitions.
    pub seasons: Vec<MapSeason>,
    /// Time-of-day traffic density curve for road vehicles: `(hour_0_24, factor)`.
    pub traffic_density_road: Vec<(f32, f32)>,
    /// Time-of-day traffic density curve for pedestrians: `(hour_0_24, factor)`.
    pub traffic_density_passenger: Vec<(f32, f32)>,
}

impl MapManifest {
    /// Parses a map scenario manifest from raw file bytes.
    ///
    /// Automatically handles UTF-16LE/BE BOM, UTF-8 BOM, UTF-8, and CP1252 encodings.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if decoding fails or numeric fields are malformed.
    pub fn parse(bytes: &[u8]) -> Result<Self, MapParseError> {
        let text = decode_bytes(bytes);
        Self::from_text(&text)
    }

    /// Parses a map scenario manifest from decoded text.
    ///
    /// # Errors
    /// Returns [`MapParseError`] if required parameters are missing or malformed.
    pub fn from_text(text: &str) -> Result<Self, MapParseError> {
        let blocks = scan_blocks(text);
        let mut manifest = MapManifest::default();

        for block in &blocks {
            match block.keyword.as_str() {
                "name" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.name = val.clone();
                    }
                }
                "friendlyname" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.friendly_name = Some(val.clone());
                    }
                }
                "description" => {
                    manifest.description = block.parameters.clone();
                }
                "version" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.version = Some(parse_u32(val, "version", block.line_number)?);
                    }
                }
                "nextidcode" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.next_id_code =
                            Some(parse_u32(val, "NextIDCode", block.line_number)?);
                    }
                }
                "moneysystem" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.money_system = Some(val.clone());
                    }
                }
                "ticketpack" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.ticket_pack = Some(val.clone());
                    }
                }
                "standarddepot" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.standard_depot = Some(val.clone());
                    }
                }
                "years" => {
                    if block.parameters.len() >= 2 {
                        let y0 = parse_i32(&block.parameters[0], "years start", block.line_number)?;
                        let y1 = parse_i32(&block.parameters[1], "years end", block.line_number)?;
                        manifest.years = Some((y0, y1));
                    }
                }
                "realyearoffset" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.real_year_offset =
                            Some(parse_i32(val, "realyearoffset", block.line_number)?);
                    }
                }
                "repair_time_min" => {
                    if let Some(val) = block.parameters.first() {
                        manifest.repair_time_min =
                            Some(parse_f32(val, "repair_time_min", block.line_number)?);
                    }
                }
                "worldcoordinates" => {
                    manifest.world_coordinates = true;
                }
                "realrail" => {
                    manifest.real_rail = true;
                }
                "dynhelperactive" => {
                    manifest.dyn_helper_active = true;
                }
                "map" => {
                    if block.parameters.len() >= 3 {
                        let x = parse_i32(&block.parameters[0], "map tile x", block.line_number)?;
                        let y = parse_i32(&block.parameters[1], "map tile y", block.line_number)?;
                        let file = block.parameters[2].clone();
                        manifest.tiles.push(MapTileEntry {
                            x,
                            y,
                            file_name: file,
                        });
                    }
                }
                "entrypoints" => {
                    // The record is line-oriented: 11 numbers and a name, where the name may start
                    // with `#` (a depot called `# Betriebshof ...`) or be empty. The generic block
                    // scanner drops such lines and would shift every later record, so the block is
                    // read from the raw lines (P-1014, MI-0020).
                    let raw = raw_block_lines(text, block.line_number);
                    if let Some(first) = raw.first() {
                        let count = parse_usize(first, "entrypoints count", block.line_number)?;
                        let records = &raw[1..];
                        for chunk in records.chunks_exact(12).take(count) {
                            let id = parse_u32(&chunk[0], "entrypoint id", block.line_number)?;
                            let id_code =
                                parse_u32(&chunk[1], "entrypoint id_code", block.line_number)?;
                            let px = parse_f32(&chunk[3], "entrypoint px", block.line_number)?;
                            let py = parse_f32(&chunk[4], "entrypoint py", block.line_number)?;
                            let pz = parse_f32(&chunk[5], "entrypoint pz", block.line_number)?;
                            let dx = parse_f32(&chunk[7], "entrypoint dx", block.line_number)?;
                            let dy = parse_f32(&chunk[8], "entrypoint dy", block.line_number)?;
                            let dz = parse_f32(&chunk[9], "entrypoint dz", block.line_number)?;
                            let name = chunk[11].clone();

                            manifest.entry_points.push(MapEntryPoint {
                                id,
                                id_code,
                                position: Vec3::new(px, py, pz),
                                direction: Vec3::new(dx, dy, dz),
                                name,
                            });
                        }
                    }
                }
                "groundtex" => {
                    if block.parameters.len() >= 5 {
                        let gtex = block.parameters[0].clone();
                        let dtex = block.parameters[1].clone();
                        let p0 =
                            parse_f32(&block.parameters[2], "groundtex p0", block.line_number)?;
                        let p1 =
                            parse_f32(&block.parameters[3], "groundtex p1", block.line_number)?;
                        let p2 =
                            parse_f32(&block.parameters[4], "groundtex p2", block.line_number)?;
                        manifest.ground_textures.push(MapGroundTex {
                            ground_texture: gtex,
                            detail_texture: dtex,
                            params: [p0, p1, p2],
                        });
                    }
                }
                "addseason" => {
                    if block.parameters.len() >= 3 {
                        let s_id =
                            parse_u32(&block.parameters[0], "addseason id", block.line_number)?;
                        let start =
                            parse_u32(&block.parameters[1], "addseason start", block.line_number)?;
                        let end =
                            parse_u32(&block.parameters[2], "addseason end", block.line_number)?;
                        manifest.seasons.push(MapSeason {
                            season_id: s_id,
                            start_day: start,
                            end_day: end,
                        });
                    }
                }
                "trafficdensity_road" if block.parameters.len() >= 2 => {
                    let h = parse_f32(
                        &block.parameters[0],
                        "trafficdensity_road hour",
                        block.line_number,
                    )?;
                    let f = parse_f32(
                        &block.parameters[1],
                        "trafficdensity_road factor",
                        block.line_number,
                    )?;
                    manifest.traffic_density_road.push((h, f));
                }
                "trafficdensity_passenger" if block.parameters.len() >= 2 => {
                    let h = parse_f32(
                        &block.parameters[0],
                        "trafficdensity_passenger hour",
                        block.line_number,
                    )?;
                    let f = parse_f32(
                        &block.parameters[1],
                        "trafficdensity_passenger factor",
                        block.line_number,
                    )?;
                    manifest.traffic_density_passenger.push((h, f));
                }
                _ => {
                    // Unknown or comment block - ignore gracefully
                }
            }
        }

        Ok(manifest)
    }
}

fn parse_f32(s: &str, field: &'static str, line: usize) -> Result<f32, MapParseError> {
    parse_float(s).map_err(|_| MapParseError::InvalidNumber {
        field,
        value: s.to_string(),
        line,
    })
}

fn parse_i32(s: &str, field: &'static str, line: usize) -> Result<i32, MapParseError> {
    s.trim()
        .parse::<i32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

/// The lines after the keyword line `keyword_line` (1-based) up to the next keyword line, trimmed
/// but otherwise untouched: blank lines and lines starting with `#` or `//` are kept.
fn raw_block_lines(text: &str, keyword_line: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.lines().skip(keyword_line) {
        let t = raw.trim();
        if raw.starts_with('[') && raw.trim_end().ends_with(']') {
            break;
        }
        out.push(t.to_string());
    }
    // the first line is the count: skip blank lines before it, as the generic scanner does
    while out.first().is_some_and(String::is_empty) {
        out.remove(0);
    }
    out
}

fn parse_u32(s: &str, field: &'static str, line: usize) -> Result<u32, MapParseError> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

fn parse_usize(s: &str, field: &'static str, line: usize) -> Result<usize, MapParseError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| MapParseError::InvalidNumber {
            field,
            value: s.to_string(),
            line,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_global_cfg() {
        let text = r#"
[name]
Grundorf

[friendlyname]
Grundorf

[description]
Grundorf is a small test map.
Enjoy driving!
[end]

[version]
14

[NextIDCode]
322886

[moneysystem]
Money\DM\DM.cfg

[ticketpack]
Ticketpacks\Berlin_1\Berlin_1.otp

[standarddepot]
Grundorf

[map]
0
0
tile_0_0.map

[map]
0
-1
tile_0_-1.map
"#;
        let manifest = MapManifest::from_text(text).unwrap();
        assert_eq!(manifest.name, "Grundorf");
        assert_eq!(manifest.friendly_name.as_deref(), Some("Grundorf"));
        assert_eq!(
            manifest.description,
            vec!["Grundorf is a small test map.", "Enjoy driving!"]
        );
        assert_eq!(manifest.version, Some(14));
        assert_eq!(manifest.next_id_code, Some(322886));
        assert_eq!(manifest.money_system.as_deref(), Some(r"Money\DM\DM.cfg"));
        assert_eq!(
            manifest.ticket_pack.as_deref(),
            Some(r"Ticketpacks\Berlin_1\Berlin_1.otp")
        );
        assert_eq!(manifest.standard_depot.as_deref(), Some("Grundorf"));
        assert_eq!(manifest.tiles.len(), 2);
        assert_eq!(manifest.tiles[0].x, 0);
        assert_eq!(manifest.tiles[0].y, 0);
        assert_eq!(manifest.tiles[0].file_name, "tile_0_0.map");
        assert_eq!(manifest.tiles[1].x, 0);
        assert_eq!(manifest.tiles[1].y, -1);
        assert_eq!(manifest.tiles[1].file_name, "tile_0_-1.map");
    }

    /// MI-0020: an entry point whose name starts with `#` or is empty must not shift the records
    /// behind it (a map with such a depot failed to load: `invalid number for entrypoint id_code`).
    #[test]
    fn test_entrypoint_names_may_start_with_hash_or_be_empty() {
        let rec = |id: u32, name: &str| {
            format!(
                "{id}\n5464087\n0\n47.929\n-0.010\n45.933\n0.000\n0.707\n0.000\n0.707\n464\n{name}\n"
            )
        };
        let text = format!(
            "[entrypoints]\n3\n{}{}{}\n[map]\n0\n0\ntile_0_0.map\n",
            rec(1, "# Betriebshof Rostorf"),
            rec(2, ""),
            rec(3, "Haltestelle")
        );
        let m = MapManifest::from_text(&text).expect("parses");
        let ids: Vec<u32> = m.entry_points.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(m.entry_points[0].name, "# Betriebshof Rostorf");
        assert_eq!(m.entry_points[1].name, "");
        assert_eq!(m.entry_points[2].name, "Haltestelle");
        assert_eq!(m.tiles.len(), 1);
    }

    #[test]
    fn test_parse_entrypoints() {
        let text = r#"
[entrypoints]
1
221
48551
0
60.113
33.200
313.728
0.000
-0.449
0.000
0.894
18
Spandau Westerwaldstr.
"#;
        let manifest = MapManifest::from_text(text).unwrap();
        assert_eq!(manifest.entry_points.len(), 1);
        let ep = &manifest.entry_points[0];
        assert_eq!(ep.id, 221);
        assert_eq!(ep.id_code, 48551);
        assert_eq!(ep.position, Vec3::new(60.113, 33.200, 313.728));
        assert_eq!(ep.direction, Vec3::new(-0.449, 0.000, 0.894));
        assert_eq!(ep.name, "Spandau Westerwaldstr.");
    }
}
