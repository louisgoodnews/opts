//! Mesh geometry, normals, UV coordinates, and material block parsers for DirectX `.x` files.

use super::material::parse_material_list;
use super::*;

pub(super) fn parse_mesh(
    reader: &mut TokenReader,
    global_mats: &HashMap<String, XMaterial>,
) -> Result<XSubMesh, XParseError> {
    let name = match reader.peek() {
        Some(t) if matches!(t.kind, TokenKind::Ident(_)) => {
            let tok = reader.next()?;
            tok.as_ident().map(|s| s.to_string())
        }
        _ => None,
    };

    reader.expect_open_brace()?;

    // Vertices
    let n_vertices = reader.parse_count()?;
    let mut positions = Vec::with_capacity(n_vertices);
    for _ in 0..n_vertices {
        let x = reader.parse_f32()?;
        let y = reader.parse_f32()?;
        let z = reader.parse_f32()?;
        positions.push(Vec3::new(x, y, z));
    }

    // Faces
    let n_faces = reader.parse_count()?;
    let mut raw_faces = Vec::with_capacity(n_faces);
    for _ in 0..n_faces {
        let count = reader.parse_count()?;
        let mut face_indices = Vec::with_capacity(count);
        for _ in 0..count {
            face_indices.push(reader.parse_u32()?);
        }
        raw_faces.push(face_indices);
    }

    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut face_materials = None;
    let mut materials = Vec::new();

    while let Some(tok) = reader.peek() {
        if tok.kind == TokenKind::CloseBrace {
            break;
        }
        match &tok.kind {
            TokenKind::Ident(id) if id == "MeshNormals" => {
                reader.pos += 1;
                normals = parse_normals(reader)?;
            }
            TokenKind::Ident(id) if id == "MeshTextureCoords" => {
                reader.pos += 1;
                uvs = parse_texture_coords(reader)?;
            }
            TokenKind::Ident(id) if id == "MeshMaterialList" => {
                reader.pos += 1;
                let (f_mats, mats) = parse_material_list(reader, global_mats)?;
                face_materials = Some(f_mats);
                materials = mats;
            }
            TokenKind::Separator(_) => {
                reader.pos += 1;
            }
            _ => {
                reader.skip_unknown_block()?;
            }
        }
    }

    reader.expect_close_brace()?;

    // Triangulate
    let (indices, tri_materials) =
        triangulate_faces_with_materials(&raw_faces, face_materials.as_deref(), positions.len())?;

    // Assemble vertices
    let mut vertices = Vec::with_capacity(positions.len());
    for (i, pos) in positions.into_iter().enumerate() {
        let norm = normals.get(i).copied().unwrap_or(Vec3::Z);
        let uv = uvs.get(i).copied().unwrap_or(Vec2::ZERO);
        vertices.push(XVertex::new(pos, norm, uv));
    }

    Ok(XSubMesh {
        name,
        vertices,
        indices,
        face_materials: tri_materials,
        materials,
    })
}

fn parse_normals(reader: &mut TokenReader) -> Result<Vec<Vec3>, XParseError> {
    reader.expect_open_brace()?;
    let n_normals = reader.parse_count()?;
    let mut normals = Vec::with_capacity(n_normals);
    for _ in 0..n_normals {
        let x = reader.parse_f32()?;
        let y = reader.parse_f32()?;
        let z = reader.parse_f32()?;
        normals.push(Vec3::new(x, y, z));
    }
    let n_face_normals = reader.parse_count()?;
    for _ in 0..n_face_normals {
        let count = reader.parse_count()?;
        for _ in 0..count {
            reader.parse_u32()?;
        }
    }
    reader.skip_separators();
    reader.expect_close_brace()?;
    Ok(normals)
}

fn parse_texture_coords(reader: &mut TokenReader) -> Result<Vec<Vec2>, XParseError> {
    reader.expect_open_brace()?;
    let n_coords = reader.parse_count()?;
    let mut uvs = Vec::with_capacity(n_coords);
    for _ in 0..n_coords {
        let u = reader.parse_f32()?;
        let v = reader.parse_f32()?;
        uvs.push(Vec2::new(u, v));
    }
    reader.skip_separators();
    reader.expect_close_brace()?;
    Ok(uvs)
}
