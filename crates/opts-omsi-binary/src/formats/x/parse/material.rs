//! Material blocks of DirectX `.x` files: the mesh material list and named `Material` objects.

use super::*;

pub(super) fn parse_material_list(
    reader: &mut TokenReader,
    global_mats: &HashMap<String, XMaterial>,
) -> Result<(Vec<u32>, Vec<XMaterial>), XParseError> {
    reader.expect_open_brace()?;
    let _n_materials = reader.parse_u32()? as usize;
    let n_faces = reader.parse_count()?;
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
                    Token {
                        kind: TokenKind::Ident(name),
                        ..
                    } => name.clone(),
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

pub(super) fn parse_material(reader: &mut TokenReader) -> Result<XMaterial, XParseError> {
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
                // Some exporters list extra maps (`"diffuse.jpg", "normal.jpg";`); only the first
                // is the colour texture.
                while !matches!(
                    reader.peek().map(|t| &t.kind),
                    Some(TokenKind::CloseBrace) | None
                ) {
                    reader.pos += 1;
                }
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
