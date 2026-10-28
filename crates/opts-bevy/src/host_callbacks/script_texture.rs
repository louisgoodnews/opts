//! Font, text-length, and script-texture (`ST*`) host callbacks for vehicle scripts.

use super::*;

pub(super) fn handle_script_texture_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        // Fonts & Metrics
        // `fontName (M.V.GetFontIndex)` -> font handle/index
        "getfontindex" => {
            let name = st.strings.pop().unwrap_or_default();
            let name_trim = name.trim();
            let Ok(fonts) = ctx.fonts.read() else {
                st.numbers.push(0.0);
                return Some(true);
            };
            let mut found = None;
            for (i, f) in fonts.iter().enumerate() {
                if f.name.eq_ignore_ascii_case(name_trim) {
                    found = Some(i as f32);
                    break;
                }
            }
            let idx = found
                .or_else(|| {
                    if name_trim.eq_ignore_ascii_case("default") && !fonts.is_empty() {
                        Some(0.0)
                    } else if !fonts.is_empty() {
                        fonts
                            .iter()
                            .position(|f| {
                                f.name
                                    .to_ascii_lowercase()
                                    .starts_with(&name_trim.to_ascii_lowercase())
                            })
                            .map(|i| i as f32)
                    } else {
                        None
                    }
                })
                .unwrap_or(0.0);
            st.numbers.push(idx);
            Some(true)
        }

        // `font text (M.V.TextLength)` -> text width in pixels
        "textlength" => {
            let font_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let t = st.strings.pop().unwrap_or_default();
            let width = if let Ok(fonts) = ctx.fonts.read() {
                if font_idx >= 0 && (font_idx as usize) < fonts.len() {
                    let f = &fonts[font_idx as usize];
                    crate::text_texture::cells_width(f, &t) as f32
                } else {
                    (t.chars().count() * 8) as f32
                }
            } else {
                (t.chars().count() * 8) as f32
            };
            st.numbers.push(width);
            Some(true)
        }

        // Script Textures (ST*)
        // `tex (M.V.STNewTex)` -> void
        "stnewtex" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.new_tex();
                    }
                }
            }
            Some(true)
        }

        // `tex (M.V.STLock)` -> void
        "stlock" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.lock();
                    }
                }
            }
            Some(true)
        }

        // `tex (M.V.STUnlock)` -> void
        "stunlock" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.unlock();
                    }
                }
            }
            Some(true)
        }

        // `tex A R G B (M.V.STSetColor)` -> void
        "stsetcolor" => {
            let b = st.numbers.pop().unwrap_or(0.0) as u8;
            let g = st.numbers.pop().unwrap_or(0.0) as u8;
            let r = st.numbers.pop().unwrap_or(0.0) as u8;
            let a = st.numbers.pop().unwrap_or(255.0) as u8;
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.set_color(a, r, g, b);
                    }
                }
            }
            Some(true)
        }

        // `tex x1 y1 x2 y2 (M.V.STDrawRect)` -> void
        "stdrawrect" => {
            let y2 = st.numbers.pop().unwrap_or(0.0) as i32;
            let x2 = st.numbers.pop().unwrap_or(0.0) as i32;
            let y1 = st.numbers.pop().unwrap_or(0.0) as i32;
            let x1 = st.numbers.pop().unwrap_or(0.0) as i32;
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.draw_rect(x1, y1, x2, y2);
                    }
                }
            }
            Some(true)
        }

        // `tex x y (M.V.STDrawPixel)` -> void
        "stdrawpixel" => {
            let y = st.numbers.pop().unwrap_or(0.0) as i32;
            let x = st.numbers.pop().unwrap_or(0.0) as i32;
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.draw_pixel(x, y);
                    }
                }
            }
            Some(true)
        }

        // `tex x y (M.V.STReadPixel)` -> void
        "streadpixel" => {
            let y = st.numbers.pop().unwrap_or(0.0) as i32;
            let x = st.numbers.pop().unwrap_or(0.0) as i32;
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.read_pixel(x, y);
                    }
                }
            }
            Some(true)
        }

        // `tex (M.V.STGetR)` -> R
        "stgetr" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let val = if tex_idx >= 0 {
                ctx.script_textures
                    .read()
                    .ok()
                    .and_then(|texs| texs.get(tex_idx as usize).map(|t| t.get_r()))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            st.numbers.push(val);
            Some(true)
        }

        // `tex (M.V.STGetG)` -> G
        "stgetg" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let val = if tex_idx >= 0 {
                ctx.script_textures
                    .read()
                    .ok()
                    .and_then(|texs| texs.get(tex_idx as usize).map(|t| t.get_g()))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            st.numbers.push(val);
            Some(true)
        }

        // `tex (M.V.STGetB)` -> B
        "stgetb" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let val = if tex_idx >= 0 {
                ctx.script_textures
                    .read()
                    .ok()
                    .and_then(|texs| texs.get(tex_idx as usize).map(|t| t.get_b()))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            st.numbers.push(val);
            Some(true)
        }

        // `tex (M.V.STGetA)` -> A
        "stgeta" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let val = if tex_idx >= 0 {
                ctx.script_textures
                    .read()
                    .ok()
                    .and_then(|texs| texs.get(tex_idx as usize).map(|t| t.get_a()))
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            st.numbers.push(val);
            Some(true)
        }

        // `tex (M.V.STFilter)` -> void
        "stfilter" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 {
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let Some(tex) = texs.get_mut(tex_idx as usize) {
                        tex.filter();
                    }
                }
            }
            Some(true)
        }

        // `file tex (M.V.STLoadTex)` -> msg
        "stloadtex" => {
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let file = st.strings.pop().unwrap_or_default();
            if tex_idx >= 0 && !file.is_empty() {
                // OMSI paths: backslashes, any letter case, relative to the vehicle's
                // texture folder (`..\..\Anzeigen\...` reaches `Vehicles/Anzeigen`).
                let path = match &ctx.texture_base_dir {
                    Some(base) => crate::script_text::resolve_path(base, &file),
                    None => Some(std::path::PathBuf::from(file.replace('\\', "/"))),
                };
                if let Ok(mut texs) = ctx.script_textures.write() {
                    if let (Some(tex), Some(path)) = (texs.get_mut(tex_idx as usize), path) {
                        let _ = tex.load_tex(&path);
                    }
                }
            }
            st.strings.push(String::new());
            Some(true)
        }

        // `tex x y font a b (text) (M.V.STTextOut)` -> void
        "sttextout" => {
            let text = st.strings.pop().unwrap_or_default();
            let spacing = st.numbers.pop().unwrap_or(0.0) as i32;
            let alignment = st.numbers.pop().unwrap_or(0.0) as u32;
            let font_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            let y = st.numbers.pop().unwrap_or(0.0) as i32;
            let x = st.numbers.pop().unwrap_or(0.0) as i32;
            let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
            if tex_idx >= 0 && font_idx >= 0 {
                if let Ok(fonts) = ctx.fonts.read() {
                    if let Some(font) = fonts.get(font_idx as usize) {
                        let bitmaps = ctx.font_bitmaps.get(font_idx as usize);
                        if let Ok(mut texs) = ctx.script_textures.write() {
                            if let Some(tex) = texs.get_mut(tex_idx as usize) {
                                match bitmaps {
                                    Some(b) => crate::script_text::draw_text(
                                        tex, x, y, font, &b, &text, spacing,
                                    ),
                                    None => tex.text_out(x, y, font, &text, alignment, spacing),
                                }
                            }
                        }
                    }
                }
            }
            Some(true)
        }

        // `tex (M.V.STCopyColor)` -> void
        "stcopycolor" => {
            let _tex = st.numbers.pop();
            Some(true)
        }

        _ => None,
    }
}
