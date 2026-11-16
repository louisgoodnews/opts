//! Tests of the scenery lights.

use super::*;

/// The glow is brightest at the centre and has no visible edge.
#[test]
fn glow_disc_fades_from_centre_to_rim() {
    let mesh = glow_mesh();
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("colours");
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions");
    };
    assert_eq!(colors[0], [1.0; 4]);
    for (p, c) in positions.iter().zip(colors) {
        let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
        if (r - 0.5).abs() < 1e-5 {
            assert_eq!(c[3], 0.0, "rim must be transparent");
        }
        assert!(r <= 0.5 + 1e-5, "unit diameter");
    }
}
