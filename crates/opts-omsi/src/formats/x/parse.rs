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

mod material;
mod mesh;
mod reader;

use material::parse_material;
use mesh::parse_mesh;
use reader::TokenReader;

/// Parses a DirectX `.x` byte buffer into a structured [`XMesh`].
///
/// # Arguments
/// * `bytes` - Raw file byte slice starting with `xof 0303txt 0032` or `0302txt`.
///
/// # Errors
/// Returns [`XParseError`] if the header is invalid, tokens are malformed, or references fail.
pub fn parse_x(bytes: &[u8]) -> Result<XMesh, XParseError> {
    let (header, payload) = parse_header(bytes)?;
    let tokens = if header.binary {
        crate::formats::x::binary::tokenize_binary(payload, header.float_bits)?
    } else {
        tokenize(payload, 16)?
    };
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
