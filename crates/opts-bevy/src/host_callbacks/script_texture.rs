//! Font, text-length, and script-texture (`ST*`) host callbacks for vehicle scripts.

use super::*;

mod pixels;
mod text;

use pixels::*;
use text::*;

pub(super) fn handle_script_texture_callback(
    ctx: &VehicleHostContext,
    name: &str,
    st: &mut Stacks,
) -> Option<bool> {
    handle_font_metrics(ctx, name, st)
        .or_else(|| handle_st_pixel_write(ctx, name, st))
        .or_else(|| handle_st_color_read(ctx, name, st))
        .or_else(|| handle_st_lifecycle_text(ctx, name, st))
}
