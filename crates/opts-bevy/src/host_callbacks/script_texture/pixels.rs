//! Pixel writes, colour reads and texture mutation for the script texture (`ST*`) callbacks.

use super::*;

pub(super) fn handle_st_pixel_write(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
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
        _ => None,
    }
}

pub(super) fn handle_st_color_read(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    match name {
        "stgetr" => {
            let val = read_channel(ctx, st, |t| t.get_r());
            st.numbers.push(val);
            Some(true)
        }
        "stgetg" => {
            let val = read_channel(ctx, st, |t| t.get_g());
            st.numbers.push(val);
            Some(true)
        }
        "stgetb" => {
            let val = read_channel(ctx, st, |t| t.get_b());
            st.numbers.push(val);
            Some(true)
        }
        "stgeta" => {
            let val = read_channel(ctx, st, |t| t.get_a());
            st.numbers.push(val);
            Some(true)
        }
        "stcopycolor" => {
            let _tex = st.numbers.pop();
            Some(true)
        }
        _ => None,
    }
}

pub(super) fn read_channel(
    ctx: &VehicleHostContext,
    st: &mut Stacks,
    f: impl FnOnce(&CpuScriptTexture) -> f32,
) -> f32 {
    let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
    if tex_idx >= 0 {
        ctx.script_textures
            .read()
            .ok()
            .and_then(|texs| texs.get(tex_idx as usize).map(f))
            .unwrap_or(0.0)
    } else {
        0.0
    }
}
pub(super) fn mutate_tex(
    ctx: &VehicleHostContext,
    st: &mut Stacks,
    f: impl FnOnce(&mut CpuScriptTexture),
) -> Option<bool> {
    let tex_idx = st.numbers.pop().unwrap_or(0.0).round() as isize;
    if tex_idx >= 0 {
        if let Ok(mut texs) = ctx.script_textures.write() {
            if let Some(tex) = texs.get_mut(tex_idx as usize) {
                f(tex);
            }
        }
    }
    Some(true)
}
