//! Main template and hierarchy parser for Microsoft DirectX `.x` files.
//!
//! Orchestrates header validation, tokenization, frame hierarchy parsing,
//! quad triangulation, and named material reference resolution.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::formats::x::ast::{XFrame, XMaterial, XMesh, XSubMesh, XVertex};
use crate::formats::x::error::XParseError;
use crate::formats::x::header::parse_header;
use crate::formats::x::lexer::{tokenize, Token, TokenKind};
use crate::formats::x::triangulate::triangulate_faces_with_materials;

struct TokenReader<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> TokenReader<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Result<&'a Token, XParseError> {
        if self.pos < self.tokens.len() {
            let tok = &self.tokens[self.pos];
            self.pos += 1;
            Ok(tok)
        } else {
            let last_offset = self.tokens.last().map(|t| t.offset).unwrap_or(0);
            Err(XParseError::UnexpectedEof(last_offset))
        }
    }

    fn skip_separators(&mut self) {
        while let Some(tok) = self.peek() {
            if matches!(tok.kind, TokenKind::Separator(_)) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect_open_brace(&mut self) -> Result<(), XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        if tok.kind == TokenKind::OpenBrace {
            Ok(())
        } else {
            Err(XParseError::UnexpectedToken {
                offset: tok.offset,
                expected: "'{'".to_string(),
                found: format!("{:?}", tok.kind),
            })
        }
    }

    fn expect_close_brace(&mut self) -> Result<(), XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        if tok.kind == TokenKind::CloseBrace {
            Ok(())
        } else {
            Err(XParseError::UnexpectedToken {
                offset: tok.offset,
                expected: "'}'".to_string(),
                found: format!("{:?}", tok.kind),
            })
        }
    }

    fn parse_f32(&mut self) -> Result<f32, XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        tok.to_f32()
    }

    fn parse_u32(&mut self) -> Result<u32, XParseError> {
        self.skip_separators();
        let tok = self.next()?;
        tok.to_u32()
    }

    fn skip_unknown_block(&mut self) -> Result<(), XParseError> {
        // Find opening brace first
        while let Some(tok) = self.peek() {
            if tok.kind == TokenKind::OpenBrace {
                break;
            }
            self.pos += 1;
        }
        self.expect_open_brace()?;
        let mut depth = 1;
        while depth > 0 {
            let tok = self.next()?;
            match tok.kind {
                TokenKind::OpenBrace => depth += 1,
                TokenKind::CloseBrace => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }
}

/// Parses a DirectX `.x` byte buffer into a structured [`XMesh`].
///
/// # Arguments
/// * `bytes` - Raw file byte slice starting with `xof 0303txt 0032` or `0302txt`.
///
/// # Errors
/// Returns [`XParseError`] if the header is invalid, tokens are malformed, or references fail.
pub fn parse_x(bytes: &[u8]) -> Result<XMesh, XParseError> {
    let (_header, text_bytes) = parse_header(bytes)?;
    let tokens = tokenize(text_bytes, 16)?;
    let mut reader = TokenReader::new(&tokens);

    let mut frames = Vec::new();
    let mut top_level_meshes = Vec::new();
    let mut global_materials = HashMap::new();

    while let Some(tok) = reader.peek() {
        match &tok.kind {
            TokenKind::Ident(id) if id == "template" => {
                reader.pos += 1; // consume 'template'
                reader.skip_unknown_block()?;
            }
            TokenKind::Ident(id) if id == "Material" => {
                reader.pos += 1; // consume 'Material'
                let mat = parse_material(&mut reader)?;
                if let Some(ref name) = mat.name {
                    global_materials.insert(name.clone(), mat);
                }
            }
            TokenKind::Ident(id) if id == "Frame" => {
                reader.pos += 1; // consume 'Frame'
                let frame = parse_frame(&mut reader, &global_materials)?;
                frames.push(frame);
            }
            TokenKind::Ident(id) if id == "Mesh" => {
                reader.pos += 1; // consume 'Mesh'
                let mesh = parse_mesh(&mut reader, &global_materials)?;
                top_level_meshes.push(mesh);
            }
            TokenKind::Separator(_) => {
                reader.pos += 1;
            }
            _ => {
                reader.skip_unknown_block()?;
            }
        }
    }

    let global_mat_list = global_materials.into_values().collect();
    Ok(XMesh {
        frames,
        top_level_meshes,
        global_materials: global_mat_list,
    })
}

fn parse_frame(
    reader: &mut TokenReader,
    global_mats: &HashMap<String, XMaterial>,
) -> Result<XFrame, XParseError> {
    let name = match reader.peek() {
        Some(t) if matches!(t.kind, TokenKind::Ident(_)) => {
            let tok = reader.next()?;
            tok.as_ident().map(|s| s.to_string())
        }
        _ => None,
    };

    reader.expect_open_brace()?;
    let mut transform = None;
    let mut meshes = Vec::new();
    let mut children = Vec::new();

    while let Some(tok) = reader.peek() {
        if tok.kind == TokenKind::CloseBrace {
            break;
        }
        match &tok.kind {
            TokenKind::Ident(id) if id == "FrameTransformMatrix" => {
                reader.pos += 1;
                transform = Some(parse_matrix(reader)?);
            }
            TokenKind::Ident(id) if id == "Mesh" => {
                reader.pos += 1;
                meshes.push(parse_mesh(reader, global_mats)?);
            }
            TokenKind::Ident(id) if id == "Frame" => {
                reader.pos += 1;
                children.push(parse_frame(reader, global_mats)?);
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
    Ok(XFrame {
        name,
        transform,
        meshes,
        children,
    })
}

fn parse_matrix(reader: &mut TokenReader) -> Result<[[f32; 4]; 4], XParseError> {
    reader.expect_open_brace()?;
    let mut m = [[0.0f32; 4]; 4];
    for row in &mut m {
        for cell in row {
            *cell = reader.parse_f32()?;
        }
    }
    reader.skip_separators();
    reader.expect_close_brace()?;
    Ok(m)
}

fn parse_mesh(
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
    let n_vertices = reader.parse_u32()? as usize;
    let mut positions = Vec::with_capacity(n_vertices);
    for _ in 0..n_vertices {
        let x = reader.parse_f32()?;
        let y = reader.parse_f32()?;
        let z = reader.parse_f32()?;
        positions.push(Vec3::new(x, y, z));
    }

    // Faces
    let n_faces = reader.parse_u32()? as usize;
    let mut raw_faces = Vec::with_capacity(n_faces);
    for _ in 0..n_faces {
        let count = reader.parse_u32()? as usize;
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
    let n_normals = reader.parse_u32()? as usize;
    let mut normals = Vec::with_capacity(n_normals);
    for _ in 0..n_normals {
        let x = reader.parse_f32()?;
        let y = reader.parse_f32()?;
        let z = reader.parse_f32()?;
        normals.push(Vec3::new(x, y, z));
    }
    let n_face_normals = reader.parse_u32()? as usize;
    for _ in 0..n_face_normals {
        let count = reader.parse_u32()? as usize;
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
    let n_coords = reader.parse_u32()? as usize;
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

fn parse_material_list(
    reader: &mut TokenReader,
    global_mats: &HashMap<String, XMaterial>,
) -> Result<(Vec<u32>, Vec<XMaterial>), XParseError> {
    reader.expect_open_brace()?;
    let _n_materials = reader.parse_u32()? as usize;
    let n_faces = reader.parse_u32()? as usize;
    let mut face_materials = Vec::with_capacity(n_faces);
    for _ in 0..n_faces {
        face_materials.push(reader.parse_u32()?);
    }

    let mut materials = Vec::new();
    while let Some(tok) = reader.peek() {
        if tok.kind == TokenKind::CloseBrace {
            break;
        }
        match &tok.kind {
            TokenKind::Ident(id) if id == "Material" => {
                reader.pos += 1;
                materials.push(parse_material(reader)?);
            }
            TokenKind::OpenBrace => {
                // Named material reference: { MaterialName }
                reader.pos += 1; // consume '{'
                let ref_name = match reader.next()? {
                    t if matches!(t.kind, TokenKind::Ident(_)) => t.as_ident().unwrap().to_string(),
                    other => {
                        return Err(XParseError::UnexpectedToken {
                            offset: other.offset,
                            expected: "material name identifier".to_string(),
                            found: format!("{:?}", other.kind),
                        })
                    }
                };
                reader.expect_close_brace()?;
                if let Some(mat) = global_mats.get(&ref_name) {
                    materials.push(mat.clone());
                } else {
                    return Err(XParseError::UnresolvedMaterialReference(ref_name));
                }
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
    Ok((face_materials, materials))
}

fn parse_material(reader: &mut TokenReader) -> Result<XMaterial, XParseError> {
    let name = match reader.peek() {
        Some(t) if matches!(t.kind, TokenKind::Ident(_)) => {
            let tok = reader.next()?;
            tok.as_ident().map(|s| s.to_string())
        }
        _ => None,
    };

    reader.expect_open_brace()?;
    let r = reader.parse_f32()?;
    let g = reader.parse_f32()?;
    let b = reader.parse_f32()?;
    let a = reader.parse_f32()?;
    let power = reader.parse_f32()?;
    let spec_r = reader.parse_f32()?;
    let spec_g = reader.parse_f32()?;
    let spec_b = reader.parse_f32()?;
    let emis_r = reader.parse_f32()?;
    let emis_g = reader.parse_f32()?;
    let emis_b = reader.parse_f32()?;

    let mut texture_filename = None;
    while let Some(tok) = reader.peek() {
        if tok.kind == TokenKind::CloseBrace {
            break;
        }
        match &tok.kind {
            TokenKind::Ident(id) if id == "TextureFilename" => {
                reader.pos += 1;
                reader.expect_open_brace()?;
                reader.skip_separators();
                let tex_tok = reader.next()?;
                if let TokenKind::String(ref s) = tex_tok.kind {
                    texture_filename = Some(s.clone());
                }
                reader.skip_separators();
                reader.expect_close_brace()?;
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
    Ok(XMaterial {
        name,
        diffuse: [r, g, b, a],
        power,
        specular: [spec_r, spec_g, spec_b],
        emissive: [emis_r, emis_g, emis_b],
        texture_filename,
    })
}
