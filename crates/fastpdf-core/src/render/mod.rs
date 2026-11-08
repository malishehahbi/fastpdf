mod interpreter;
mod rasterize;

use crate::error::Result;
use crate::types::RenderOutput;
use interpreter::interpret_page;
use lopdf::{Document, ObjectId};
use rasterize::rasterize;

pub fn render_page(
    doc: &Document,
    page_id: ObjectId,
    page_width: f32,
    page_height: f32,
    scale: f32,
) -> Result<RenderOutput> {
    let items = interpret_page(doc, page_id, page_width, page_height)?;
    let width = (page_width * scale).ceil().max(1.0) as u32;
    let height = (page_height * scale).ceil().max(1.0) as u32;
    let pixels = rasterize(&items, page_width, page_height, width, height, scale)?;
    Ok(RenderOutput {
        width,
        height,
        pixels,
    })
}

pub fn render_tile(
    doc: &Document,
    page_id: ObjectId,
    page_width: f32,
    page_height: f32,
    scale: f32,
    tile_x: f32,
    tile_y: f32,
    tile_w: f32,
    tile_h: f32,
) -> Result<RenderOutput> {
    let items = interpret_page(doc, page_id, page_width, page_height)?;
    let width = (tile_w * scale).ceil().max(1.0) as u32;
    let height = (tile_h * scale).ceil().max(1.0) as u32;
    let pixels = rasterize::rasterize_tile(
        &items,
        page_width,
        page_height,
        width,
        height,
        scale,
        tile_x,
        tile_y,
        tile_w,
        tile_h,
    )?;
    Ok(RenderOutput {
        width,
        height,
        pixels,
    })
}
