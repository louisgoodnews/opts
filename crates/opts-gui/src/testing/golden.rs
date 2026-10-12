//! Golden screenshots without a GPU: a software painter for the laid-out UI tree and a
//! tolerance-based image comparison.
//!
//! **What this is.** [`paint`] draws what Bevy's UI layout computed for a headless app: node
//! backgrounds with rounded corners, borders and the real glyphs of every text (the font atlas
//! Bevy rasterises on the CPU), in paint order. It is deterministic and needs no window, GPU or
//! display, so it runs in CI. **What it is not.** It is not a pixel-exact copy of the GPU
//! renderer: images, shadows, outlines (the focus ring), gradients and clipping details are not
//! painted. It guards layout, colour and text regressions; the GPU screenshots of the control
//! protocol (`output.screenshot`) remain the proof of the real frame, which is an open item for a
//! human run.
//!
//! **Goldens contain fixture content only** (self-authored catalogue and texts; ADR-0002 forbids
//! OMSI content in the repository). [`check`] compares with a [`Tolerance`] and, with
//! `UPDATE_GOLDEN=1`, (re)writes the golden file.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::ui::{ComputedNode, ComputedStackIndex, UiGlobalTransform};
use image::{Rgba, RgbaImage};

/// How far an image may differ from its golden and still pass.
#[derive(Debug, Clone, Copy)]
pub struct Tolerance {
    /// Largest allowed difference per colour channel (0 to 255) before a pixel counts as different.
    pub channel: u8,
    /// Largest allowed share of different pixels (0.0 to 1.0).
    pub fraction: f32,
}

impl Default for Tolerance {
    fn default() -> Self {
        Self {
            channel: 6,
            fraction: 0.002,
        }
    }
}

/// Result of [`compare`].
#[derive(Debug, Clone, PartialEq)]
pub struct Difference {
    /// Pixels beyond the channel tolerance.
    pub different: u64,
    /// Total pixels compared.
    pub total: u64,
    /// Largest channel difference seen.
    pub max_channel: u8,
    /// The two images have different sizes (then nothing else is meaningful).
    pub size_mismatch: bool,
}

impl Difference {
    /// Share of different pixels.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.different as f32 / self.total as f32
        }
    }
}

/// Compares two images.
pub fn compare(actual: &RgbaImage, golden: &RgbaImage, tol: Tolerance) -> Difference {
    if actual.dimensions() != golden.dimensions() {
        return Difference {
            different: u64::MAX,
            total: 0,
            max_channel: 255,
            size_mismatch: true,
        };
    }
    let mut different = 0;
    let mut max_channel = 0u8;
    for (a, g) in actual.pixels().zip(golden.pixels()) {
        let d =
            a.0.iter()
                .zip(g.0.iter())
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
        max_channel = max_channel.max(d);
        if d > tol.channel {
            different += 1;
        }
    }
    Difference {
        different,
        total: u64::from(actual.width()) * u64::from(actual.height()),
        max_channel,
        size_mismatch: false,
    }
}

/// Does `d` pass `tol`?
pub fn passes(d: &Difference, tol: Tolerance) -> bool {
    !d.size_mismatch && d.fraction() <= tol.fraction
}

/// Compares `actual` with `dir/name.png`. With `UPDATE_GOLDEN=1` the golden is written instead.
/// On a mismatch the actual image and a difference image are written to
/// `$OPTS_GOLDEN_OUT` (default: `dir/_failed`) and an error describing the numbers is returned.
pub fn check(dir: &Path, name: &str, actual: &RgbaImage, tol: Tolerance) -> Result<(), String> {
    let path = dir.join(format!("{name}.png"));
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        actual.save(&path).map_err(|e| e.to_string())?;
        return Ok(());
    }
    let golden = image::open(&path)
        .map_err(|e| {
            format!(
                "golden {} missing or unreadable ({e}); run with UPDATE_GOLDEN=1 to create it",
                path.display()
            )
        })?
        .to_rgba8();
    let d = compare(actual, &golden, tol);
    if passes(&d, tol) {
        return Ok(());
    }
    let out: PathBuf = std::env::var_os("OPTS_GOLDEN_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.join("_failed"));
    let _ = std::fs::create_dir_all(&out);
    let _ = actual.save(out.join(format!("{name}.actual.png")));
    if !d.size_mismatch {
        let mut diff = RgbaImage::new(actual.width(), actual.height());
        for (x, y, p) in diff.enumerate_pixels_mut() {
            let a = actual.get_pixel(x, y);
            let g = golden.get_pixel(x, y);
            let delta =
                a.0.iter()
                    .zip(g.0.iter())
                    .map(|(p, q)| p.abs_diff(*q))
                    .max()
                    .unwrap_or(0);
            *p = if delta > tol.channel {
                Rgba([255, 0, 0, 255])
            } else {
                Rgba([g.0[0] / 2 + 64, g.0[1] / 2 + 64, g.0[2] / 2 + 64, 255])
            };
        }
        let _ = diff.save(out.join(format!("{name}.diff.png")));
    }
    Err(format!(
        "{name}: {} of {} pixels differ (max channel delta {}, allowed {:.3}%), size mismatch {}; see {}",
        d.different,
        d.total,
        d.max_channel,
        tol.fraction * 100.0,
        d.size_mismatch,
        out.display()
    ))
}

fn to_rgba(c: Color) -> [f32; 4] {
    let s = c.to_srgba();
    [s.red, s.green, s.blue, s.alpha]
}

fn blend(img: &mut RgbaImage, x: i64, y: i64, src: [f32; 4], coverage: f32) {
    if x < 0 || y < 0 || x >= i64::from(img.width()) || y >= i64::from(img.height()) {
        return;
    }
    let a = (src[3] * coverage).clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let p = img.get_pixel_mut(x as u32, y as u32);
    for i in 0..3 {
        let below = f32::from(p.0[i]) / 255.0;
        p.0[i] = ((src[i] * a + below * (1.0 - a)) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    p.0[3] = 255;
}

/// Is the point inside the rounded rectangle `min..max` with the corner radii of `r`?
fn inside_rounded(p: Vec2, min: Vec2, max: Vec2, r: &bevy::ui::ResolvedBorderRadius) -> bool {
    if p.x < min.x || p.y < min.y || p.x >= max.x || p.y >= max.y {
        return false;
    }
    let corner = |radius: Vec2, center: Vec2| -> bool {
        let rad = radius.x.max(radius.y);
        if rad <= 0.5 {
            return true;
        }
        (p - center).length() <= rad
    };
    let (tl, tr, br, bl) = (r.top_left, r.top_right, r.bottom_right, r.bottom_left);
    let c = |v: Vec2| v.x.max(v.y);
    if p.x < min.x + c(tl) && p.y < min.y + c(tl) {
        return corner(tl, min + Vec2::splat(c(tl)));
    }
    if p.x >= max.x - c(tr) && p.y < min.y + c(tr) {
        return corner(tr, Vec2::new(max.x - c(tr), min.y + c(tr)));
    }
    if p.x >= max.x - c(br) && p.y >= max.y - c(br) {
        return corner(br, max - Vec2::splat(c(br)));
    }
    if p.x < min.x + c(bl) && p.y >= max.y - c(bl) {
        return corner(bl, Vec2::new(min.x + c(bl), max.y - c(bl)));
    }
    true
}

fn is_hidden(world: &World, entity: Entity) -> bool {
    let mut cur = Some(entity);
    while let Some(e) = cur {
        if world
            .get::<Node>(e)
            .is_some_and(|n| n.display == Display::None)
            || world
                .get::<Visibility>(e)
                .is_some_and(|v| *v == Visibility::Hidden)
        {
            return true;
        }
        cur = world.get::<ChildOf>(e).map(|c| c.parent());
    }
    false
}

/// Paints the laid-out UI of `world` into an image of `width` x `height` physical pixels over
/// `background`. Text uses the glyphs Bevy rasterised; nodes are painted in stack order.
pub fn paint(world: &mut World, width: u32, height: u32, background: Color) -> RgbaImage {
    let bg = to_rgba(background);
    let mut img = RgbaImage::from_pixel(
        width,
        height,
        Rgba([
            (bg[0] * 255.0).round() as u8,
            (bg[1] * 255.0).round() as u8,
            (bg[2] * 255.0).round() as u8,
            255,
        ]),
    );
    let mut q = world.query::<(
        Entity,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedStackIndex,
    )>();
    let mut nodes: Vec<(u32, Entity)> = q.iter(world).map(|(e, _, _, s)| (s.0, e)).collect();
    nodes.sort_by_key(|(s, e)| (*s, *e));
    for (_, e) in nodes {
        if is_hidden(world, e) {
            continue;
        }
        let (Some(c), Some(t)) = (
            world.get::<ComputedNode>(e),
            world.get::<UiGlobalTransform>(e),
        ) else {
            continue;
        };
        let size = c.size();
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let center = t.translation;
        let min = center - size / 2.0;
        let max = center + size / 2.0;
        let clip = clip_of(world, e);
        let radius = c.border_radius();
        let background = world.get::<BackgroundColor>(e).map(|b| to_rgba(b.0));
        let border = world.get::<BorderColor>(e).map(|b| {
            [
                to_rgba(b.top),
                to_rgba(b.right),
                to_rgba(b.bottom),
                to_rgba(b.left),
            ]
        });
        let b = c.border();
        let inner_min = min + Vec2::new(b.min_inset.x, b.min_inset.y);
        let inner_max = max - Vec2::new(b.max_inset.x, b.max_inset.y);
        let has_border = border.is_some_and(|c| c.iter().any(|s| s[3] > 0.0))
            && (b.min_inset != Vec2::ZERO || b.max_inset != Vec2::ZERO);
        let shrink = |v: Vec2, d: Vec2| (v - d).max(Vec2::ZERO);
        let inner_radius = bevy::ui::ResolvedBorderRadius {
            top_left: shrink(radius.top_left, b.min_inset),
            top_right: shrink(radius.top_right, Vec2::new(b.max_inset.x, b.min_inset.y)),
            bottom_right: shrink(radius.bottom_right, b.max_inset),
            bottom_left: shrink(radius.bottom_left, Vec2::new(b.min_inset.x, b.max_inset.y)),
        };
        if background.is_some_and(|c| c[3] > 0.0) || has_border {
            for y in (min.y.floor() as i64)..(max.y.ceil() as i64) {
                for x in (min.x.floor() as i64)..(max.x.ceil() as i64) {
                    let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    if !inside_rounded(p, min, max, &radius) || !in_clip(clip, x, y) {
                        continue;
                    }
                    let in_border =
                        has_border && !inside_rounded(p, inner_min, inner_max, &inner_radius);
                    if in_border {
                        if let Some(sides) = border {
                            let d = [
                                p.y - min.y,
                                max.x - p.x,
                                max.y - p.y,
                                p.x - min.x,
                            ];
                            let side = (0..4)
                                .min_by(|a, b| d[*a].total_cmp(&d[*b]))
                                .unwrap_or(0);
                            blend(&mut img, x, y, sides[side], 1.0);
                        }
                    } else if let Some(col) = background {
                        blend(&mut img, x, y, col, 1.0);
                    }
                }
            }
        }
        paint_text(world, e, &mut img, clip);
    }
    img
}

/// The rectangle (physical pixels) every pixel of `e` is clipped to by its ancestors, if any.
fn clip_of(world: &World, e: Entity) -> Option<(Vec2, Vec2)> {
    let mut clip: Option<(Vec2, Vec2)> = None;
    let mut cur = world.get::<ChildOf>(e).map(|c| c.parent());
    while let Some(a) = cur {
        if let (Some(n), Some(c), Some(t)) = (
            world.get::<Node>(a),
            world.get::<ComputedNode>(a),
            world.get::<UiGlobalTransform>(a),
        ) {
            let (cx, cy) = (
                n.overflow.x != OverflowAxis::Visible,
                n.overflow.y != OverflowAxis::Visible,
            );
            if cx || cy {
                let mut lo = t.translation - c.size() / 2.0;
                let mut hi = t.translation + c.size() / 2.0;
                if !cx {
                    lo.x = f32::MIN;
                    hi.x = f32::MAX;
                }
                if !cy {
                    lo.y = f32::MIN;
                    hi.y = f32::MAX;
                }
                clip = Some(match clip {
                    Some((l, h)) => (l.max(lo), h.min(hi)),
                    None => (lo, hi),
                });
            }
        }
        cur = world.get::<ChildOf>(a).map(|c| c.parent());
    }
    clip
}

fn in_clip(clip: Option<(Vec2, Vec2)>, x: i64, y: i64) -> bool {
    clip.is_none_or(|(lo, hi)| {
        let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
        p.x >= lo.x && p.y >= lo.y && p.x < hi.x && p.y < hi.y
    })
}

fn paint_text(world: &World, e: Entity, img: &mut RgbaImage, clip: Option<(Vec2, Vec2)>) {
    let (Some(layout), Some(color), Some(c), Some(t)) = (
        world.get::<bevy::text::TextLayoutInfo>(e),
        world.get::<TextColor>(e),
        world.get::<ComputedNode>(e),
        world.get::<UiGlobalTransform>(e),
    ) else {
        return;
    };
    let origin = t.translation + c.content_box().min;
    let src = to_rgba(color.0);
    let images = world.resource::<Assets<Image>>();
    for glyph in &layout.glyphs {
        let Some(atlas) = images.get(glyph.atlas_info.texture) else {
            continue;
        };
        let rect = glyph.atlas_info.rect;
        let size = rect.size();
        let top_left = origin + glyph.position - size / 2.0;
        let red_only = matches!(
            atlas.texture_descriptor.format,
            bevy::render::render_resource::TextureFormat::R8Unorm
        );
        for gy in 0..size.y as u32 {
            for gx in 0..size.x as u32 {
                let ax = rect.min.x as u32 + gx;
                let ay = rect.min.y as u32 + gy;
                let Ok(px) = atlas.get_color_at(ax, ay) else {
                    continue;
                };
                let s = px.to_srgba();
                let coverage = if red_only { s.red } else { s.alpha };
                let (x, y) = (
                    top_left.x.round() as i64 + i64::from(gx),
                    top_left.y.round() as i64 + i64::from(gy),
                );
                if in_clip(clip, x, y) {
                    blend(img, x, y, src, coverage);
                }
            }
        }
    }
}
