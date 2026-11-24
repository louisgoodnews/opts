//! Numeric values of text files: decimal commas tolerated, and 3D vectors.

use glam::Vec3;

/// Helper to parse a floating point number tolerating commas as decimal separators.
pub fn parse_float(s: &str) -> Result<f32, std::num::ParseFloatError> {
    let normalized = s.trim().replace(',', ".");
    normalized.parse::<f32>()
}

/// Helper to parse a 64-bit floating point number tolerating commas as decimal separators.
pub fn parse_f64(s: &str) -> Result<f64, std::num::ParseFloatError> {
    let normalized = s.trim().replace(',', ".");
    normalized.parse::<f64>()
}

/// Helper to parse a 3D coordinate vector (`Vec3`) from three consecutive lines.
pub fn parse_vec3(slice: &[String]) -> Result<Vec3, String> {
    if slice.len() < 3 {
        return Err("Expected 3 lines for Vec3".to_string());
    }
    let x = parse_float(&slice[0]).map_err(|e| format!("Invalid X coord: {e}"))?;
    let y = parse_float(&slice[1]).map_err(|e| format!("Invalid Y coord: {e}"))?;
    let z = parse_float(&slice[2]).map_err(|e| format!("Invalid Z coord: {e}"))?;
    Ok(Vec3::new(x, y, z))
}
