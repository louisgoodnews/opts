//! The formation file format: the versioned TOML encoding of a formation and its decoding, with the checks of a file.

use super::*;

#[derive(Debug, Serialize, Deserialize)]
struct FileV1 {
    format: u32,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    template: Option<String>,
    #[serde(default, rename = "segment")]
    segments: Vec<SegmentV1>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SegmentV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vehicle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(default = "forward")]
    orientation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    powered: Option<bool>,
}

fn forward() -> String {
    "forward".to_owned()
}

/// The TOML text of a formation.
///
/// # Errors
/// [`StoreError::Corrupt`] when the text cannot be produced.
pub fn encode(f: &TrainFormation) -> Result<String, StoreError> {
    let file = FileV1 {
        format: FORMAT,
        name: f.name.clone(),
        template: match &f.source {
            FormationSource::ZugFile(p) => Some(p.clone()),
            FormationSource::Custom => None,
        },
        segments: f
            .segments
            .iter()
            .map(|s| SegmentV1 {
                vehicle: matches!(s.vehicle, VehicleRef::File(_))
                    .then(|| s.vehicle.as_str().to_owned()),
                group: matches!(s.vehicle, VehicleRef::Group(_))
                    .then(|| s.vehicle.as_str().to_owned()),
                orientation: match s.orientation {
                    Orientation::Forward => "forward".to_owned(),
                    Orientation::Backward => "backward".to_owned(),
                },
                powered: s.powered,
            })
            .collect(),
    };
    toml::to_string_pretty(&file).map_err(|e| StoreError::Corrupt(e.to_string()))
}

/// The formation a TOML text holds.
///
/// # Errors
/// [`StoreError`] for text that is not a formation file, one from a newer version, or one with
/// more than [`MAX_CARS`] cars.
pub fn decode(text: &str) -> Result<TrainFormation, StoreError> {
    let file: FileV1 = toml::from_str(text).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    if file.format > FORMAT {
        return Err(StoreError::Newer(file.format));
    }
    if file.format == 0 {
        return Err(StoreError::Corrupt("format must be 1".to_owned()));
    }
    if file.segments.len() > MAX_CARS {
        return Err(StoreError::Corrupt(format!("more than {MAX_CARS} cars")));
    }
    let mut segments = Vec::with_capacity(file.segments.len());
    for (i, s) in file.segments.iter().enumerate() {
        let vehicle = match (&s.vehicle, &s.group) {
            (Some(v), None) if !v.trim().is_empty() => VehicleRef::File(v.trim().to_owned()),
            (None, Some(g)) if !g.trim().is_empty() => VehicleRef::Group(g.trim().to_owned()),
            _ => {
                return Err(StoreError::Corrupt(format!(
                    "segment {}: give exactly one of `vehicle` and `group`",
                    i + 1
                )))
            }
        };
        let orientation = match s.orientation.to_ascii_lowercase().as_str() {
            "forward" => Orientation::Forward,
            "backward" => Orientation::Backward,
            other => {
                return Err(StoreError::Corrupt(format!(
                    "segment {}: orientation `{other}` is not forward or backward",
                    i + 1
                )))
            }
        };
        segments.push(FormationSegment {
            vehicle,
            orientation,
            powered: s.powered,
        });
    }
    Ok(TrainFormation {
        name: file.name,
        segments,
        source: file
            .template
            .map_or(FormationSource::Custom, FormationSource::ZugFile),
    })
}
