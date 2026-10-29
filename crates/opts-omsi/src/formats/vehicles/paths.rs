//! Parser and data model for OMSI passenger path networks (`paths.cfg`).
//!
//! A paths configuration file defines the directed graph of waypoints that
//! passengers follow through a vehicle segment from entry to exit.
//!
//! See specification in `docs/src/formats/text/vehicles/paths_cfg.md`.

use glam::Vec3;

use super::error::PathsParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

/// A node in the passenger path graph representing a 3D position inside the vehicle.
#[derive(Debug, Clone, PartialEq)]
pub struct PathPoint {
    /// Positional index assigned in declaration order (0, 1, 2, ...).
    pub index: usize,
    /// 3D coordinates in segment-local vehicle space (X right, Y forward, Z up).
    pub position: Vec3,
}

/// A directed edge connecting two pathpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLink {
    /// Source pathpoint index.
    pub from: usize,
    /// Destination pathpoint index.
    pub to: usize,
    /// `[pathlink_oneway]`: passengers may walk only from `from` to `to` (entrances lead in,
    /// exits lead out).
    pub one_way: bool,
}

/// A set of footstep audio sample files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSoundPack {
    /// List of relative `.wav` sound paths.
    pub sounds: Vec<String>,
}

/// A door binding linking a pathpoint to an entry or exit door index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorBinding {
    /// The pathpoint index at the door.
    pub pathpoint: usize,
    /// The door index.
    pub door_id: usize,
}

/// The complete passenger navigation network for a vehicle segment.
#[derive(Debug, Clone, PartialEq)]
pub struct PassengerPaths {
    /// Declared footstep sound packs.
    pub sound_packs: Vec<StepSoundPack>,
    /// Sequential pathpoints (nodes).
    pub points: Vec<PathPoint>,
    /// Directed connections (edges).
    pub links: Vec<PathLink>,
    /// Door entry bindings (if declared).
    pub entries: Vec<DoorBinding>,
    /// Door exit bindings (if declared).
    pub exits: Vec<DoorBinding>,
    /// Index into `sound_packs` for default footsteps.
    pub next_stepsound: Option<usize>,
    /// Interior ceiling/room height in meters.
    pub next_roomheight: Option<f32>,
}

fn parse_stepsoundpack(
    block: &crate::formats::text::OmsiBlock,
) -> Result<StepSoundPack, PathsParseError> {
    if block.parameters.is_empty() {
        return Err(PathsParseError::MissingParameters {
            keyword: "stepsoundpack".to_string(),
            line: block.line_number,
            expected: 1,
            found: 0,
        });
    }
    let count =
        block.parameters[0]
            .parse::<usize>()
            .map_err(|_| PathsParseError::InvalidNumber {
                line: block.line_number,
                value: block.parameters[0].clone(),
            })?;
    let sounds = block.parameters[1..].iter().take(count).cloned().collect();
    Ok(StepSoundPack { sounds })
}

fn parse_pathpoint(
    block: &crate::formats::text::OmsiBlock,
    index: usize,
) -> Result<PathPoint, PathsParseError> {
    if block.parameters.len() < 3 {
        return Err(PathsParseError::MissingParameters {
            keyword: block.keyword.clone(),
            line: block.line_number,
            expected: 3,
            found: block.parameters.len(),
        });
    }
    let x = parse_float(&block.parameters[0]).map_err(|_| PathsParseError::InvalidNumber {
        line: block.line_number,
        value: block.parameters[0].clone(),
    })?;
    let y = parse_float(&block.parameters[1]).map_err(|_| PathsParseError::InvalidNumber {
        line: block.line_number,
        value: block.parameters[1].clone(),
    })?;
    let z = parse_float(&block.parameters[2]).map_err(|_| PathsParseError::InvalidNumber {
        line: block.line_number,
        value: block.parameters[2].clone(),
    })?;
    Ok(PathPoint {
        index,
        position: Vec3::new(x, y, z),
    })
}

fn parse_pathlink(block: &crate::formats::text::OmsiBlock) -> Result<PathLink, PathsParseError> {
    if block.parameters.len() < 2 {
        return Err(PathsParseError::MissingParameters {
            keyword: block.keyword.clone(),
            line: block.line_number,
            expected: 2,
            found: block.parameters.len(),
        });
    }
    let from =
        block.parameters[0]
            .parse::<usize>()
            .map_err(|_| PathsParseError::InvalidNumber {
                line: block.line_number,
                value: block.parameters[0].clone(),
            })?;
    let to = block.parameters[1]
        .parse::<usize>()
        .map_err(|_| PathsParseError::InvalidNumber {
            line: block.line_number,
            value: block.parameters[1].clone(),
        })?;
    Ok(PathLink {
        from,
        to,
        one_way: block.keyword == "pathlink_oneway",
    })
}

fn parse_door_binding(block: &crate::formats::text::OmsiBlock) -> Option<DoorBinding> {
    if block.parameters.len() >= 2 {
        let pathpoint = block.parameters[0].parse::<usize>().unwrap_or(0);
        let door_id = block.parameters[1].parse::<usize>().unwrap_or(0);
        Some(DoorBinding { pathpoint, door_id })
    } else {
        None
    }
}

impl PassengerPaths {
    /// Parses a `paths.cfg` document from a string.
    ///
    /// # Errors
    /// Returns [`PathsParseError`] if parameter counts are invalid or numbers fail to parse.
    pub fn parse(text: &str) -> Result<Self, PathsParseError> {
        let blocks = scan_blocks(text);

        let mut sound_packs = Vec::new();
        let mut points = Vec::new();
        let mut links = Vec::new();
        let mut entries = Vec::new();
        let mut exits = Vec::new();
        let mut next_stepsound = None;
        let mut next_roomheight = None;

        for block in blocks {
            match block.keyword.as_str() {
                "stepsoundpack" => sound_packs.push(parse_stepsoundpack(&block)?),
                "pathpnt" | "pathpoint" => {
                    let point = parse_pathpoint(&block, points.len())?;
                    points.push(point);
                }
                "pathlink" | "pathlink_oneway" => links.push(parse_pathlink(&block)?),
                "entry" => {
                    if let Some(binding) = parse_door_binding(&block) {
                        entries.push(binding);
                    }
                }
                "exit" => {
                    if let Some(binding) = parse_door_binding(&block) {
                        exits.push(binding);
                    }
                }
                "next_stepsound" => {
                    if let Some(param) = block.parameters.first() {
                        let idx =
                            param
                                .parse::<usize>()
                                .map_err(|_| PathsParseError::InvalidNumber {
                                    line: block.line_number,
                                    value: param.clone(),
                                })?;
                        next_stepsound = Some(idx);
                    }
                }
                "next_roomheight" => {
                    if let Some(param) = block.parameters.first() {
                        let h = parse_float(param).map_err(|_| PathsParseError::InvalidNumber {
                            line: block.line_number,
                            value: param.clone(),
                        })?;
                        next_roomheight = Some(h);
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            sound_packs,
            points,
            links,
            entries,
            exits,
            next_stepsound,
            next_roomheight,
        })
    }

    /// Parses a `paths.cfg` document from raw bytes.
    ///
    /// # Errors
    /// Returns [`PathsParseError`] on syntax error.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, PathsParseError> {
        let text = decode_bytes(bytes);
        Self::parse(&text)
    }

    /// Returns the number of pathpoints defined in the graph.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of directed links in the graph.
    #[must_use]
    pub fn link_count(&self) -> usize {
        self.links.len()
    }
}

/// Convenience function to parse `paths.cfg` from a string slice.
pub fn parse_paths(text: &str) -> Result<PassengerPaths, PathsParseError> {
    PassengerPaths::parse(text)
}

#[cfg(test)]
mod tests;
