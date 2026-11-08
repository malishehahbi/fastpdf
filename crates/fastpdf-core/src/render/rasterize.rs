use crate::error::{FastPdfError, Result};
use crate::render::fonts::{default_font, draw_text_on_pixmap, font_from_bytes};
use crate::render::interpreter::DisplayItem;
use ab_glyph::FontArc;
use tiny_skia::{Paint, PathBuilder, Pixmap, Stroke, Transform};

pub fn rasterize(
    items: &[DisplayItem],
    page_width: f32,
    page_height: f32,
    out_w: u32,
    out_h: u32,
    scale: f32,
) -> Result<Vec<u8>> {
    rasterize_region(items, page_width, page_height, out_w, out_h, scale, 0.0, 0.0, page_width, page_height)
}

pub fn rasterize_tile(
    items: &[DisplayItem],
    page_width: f32,
    page_height: f32,
    out_w: u32,
    out_h: u32,
    scale: f32,
    tile_x: f32,
    tile_y: f32,
    tile_w: f32,
    tile_h: f32,
) -> Result<Vec<u8>> {
    rasterize_region(
        items,
        page_width,
        page_height,
        out_w,
        out_h,
        scale,
        tile_x,
        tile_y,
        tile_w,
        tile_h,
    )
}

fn rasterize_region(
    items: &[DisplayItem],
    page_width: f32,
    page_height: f32,
    out_w: u32,
    out_h: u32,
    scale: f32,
    region_x: f32,
    region_y: f32,
    region_w: f32,
    region_h: f32,
) -> Result<Vec<u8>> {
    let mut pixmap = Pixmap::new(out_w, out_h).ok_or(FastPdfError::CorruptPdf("pixmap alloc".into()))?;
    pixmap.fill(tiny_skia::Color::WHITE);

    let flip = Transform::from_row(1.0, 0.0, 0.0, -1.0, 0.0, page_height);
    let region_offset = Transform::from_translate(-region_x * scale, -(page_height - region_y - region_h) * scale);
    let view_scale = Transform::from_scale(scale, scale).pre_concat(region_offset);

    let default = default_font();

    for item in items {
        match item {
            DisplayItem::FillPath(path, color) => {
                if let Some(p) = path_to_skia(path, flip) {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                        255,
                    );
                    paint.anti_alias = true;
                    pixmap.fill_path(
                        &p,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        view_scale,
                        None,
                    );
                }
            }
            DisplayItem::StrokePath(path, color, width) => {
                if let Some(p) = path_to_skia(path, flip) {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(
                        (color[0] * 255.0) as u8,
                        (color[1] * 255.0) as u8,
                        (color[2] * 255.0) as u8,
                        255,
                    );
                    paint.anti_alias = true;
                    let stroke = Stroke {
                        width: width * scale,
                        ..Default::default()
                    };
                    pixmap.stroke_path(&p, &paint, &stroke, view_scale, None);
                }
            }
            DisplayItem::Image {
                pixels,
                width,
                height,
                ctm,
            } => {
                if let Some(size) = tiny_skia::IntSize::from_wh(*width, *height) {
                    if let Some(src) = Pixmap::from_vec(pixels.clone(), size) {
                        // PDF paints images into a 1×1 unit square mapped by the CTM at Do.
                        let img_norm = Transform::from_scale(1.0 / *width as f32, 1.0 / *height as f32);
                        let transform = flip
                            .pre_concat(mul_skia(*ctm))
                            .pre_concat(img_norm)
                            .pre_concat(view_scale);
                        pixmap.draw_pixmap(
                            0,
                            0,
                            src.as_ref(),
                            &tiny_skia::PixmapPaint::default(),
                            transform,
                            None,
                        );
                    }
                }
            }
            DisplayItem::Text {
                text,
                ctm,
                size,
                color,
                font_embed,
            } => {
                let embedded = font_embed
                    .as_ref()
                    .and_then(|b| font_from_bytes(b.clone()));
                let font: &FontArc = embedded.as_ref().unwrap_or(default);
                let mut rgb = *color;
                if rgb[0] + rgb[1] + rgb[2] > 2.85 {
                    rgb = [0.05, 0.05, 0.05];
                }
                draw_text_on_pixmap(
                    &mut pixmap,
                    text,
                    ctm[4],
                    ctm[5],
                    *size,
                    rgb,
                    page_height,
                    scale,
                    font,
                );
            }
        }
    }
    Ok(pixmap.data().to_vec())
}

fn path_to_skia(path: &[(f32, f32)], flip: Transform) -> Option<tiny_skia::Path> {
    if path.is_empty() {
        return None;
    }
    let mut pb = PathBuilder::new();
    let (x0, y0) = apply_t(flip, path[0].0, path[0].1);
    pb.move_to(x0, y0);
    for &(x, y) in &path[1..] {
        let (tx, ty) = apply_t(flip, x, y);
        pb.line_to(tx, ty);
    }
    pb.close();
    pb.finish()
}

fn apply_t(t: Transform, x: f32, y: f32) -> (f32, f32) {
    (
        t.sx * x + t.kx * y + t.tx,
        t.ky * x + t.sy * y + t.ty,
    )
}

fn mul_skia(m: [f32; 6]) -> Transform {
    Transform::from_row(m[0], m[1], m[2], m[3], m[4], m[5])
}
