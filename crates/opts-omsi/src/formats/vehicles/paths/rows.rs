//! Row parsers of `*.pass`-style path files: step sounds, path points, links and door bindings.

use super::*;

pub(super) fn parse_stepsoundpack(
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

pub(super) fn parse_pathpoint(
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

pub(super) fn parse_pathlink(
    block: &crate::formats::text::OmsiBlock,
) -> Result<PathLink, PathsParseError> {
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

pub(super) fn parse_door_binding(block: &crate::formats::text::OmsiBlock) -> Option<DoorBinding> {
    if block.parameters.len() >= 2 {
        let pathpoint = block.parameters[0].parse::<usize>().unwrap_or(0);
        let door_id = block.parameters[1].parse::<usize>().unwrap_or(0);
        Some(DoorBinding { pathpoint, door_id })
    } else {
        None
    }
}
