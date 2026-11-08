use ab_glyph::{Font, FontArc, FontRef, PxScale, ScaleFont};
use lopdf::{Document, Dictionary, Object, ObjectId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tiny_skia::Pixmap;

static DEFAULT_FONT: OnceLock<FontArc> = OnceLock::new();

pub fn default_font() -> &'static FontArc {
    DEFAULT_FONT.get_or_init(|| {
        let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/default_font.ttf"));
        FontArc::try_from_slice(bytes).expect("embedded default font")
    })
}

pub fn font_from_bytes(bytes: Arc<Vec<u8>>) -> Option<FontArc> {
    FontArc::try_from_vec((*bytes).clone()).ok()
}

pub fn extract_font_bytes(doc: &Document, font_dict: &Dictionary) -> Option<Arc<Vec<u8>>> {
    if let Ok(descendants) = font_dict.get(b"DescendantFonts") {
        if let Object::Array(arr) = descendants {
            if let Some(df_ref) = arr.first().and_then(|o| o.as_reference().ok()) {
                if let Ok(df) = doc.get_dictionary(df_ref) {
                    if let Some(b) = font_descriptor_bytes(doc, &df) {
                        return Some(Arc::new(b));
                    }
                }
            }
        }
    }
    font_descriptor_bytes(doc, font_dict).map(|b| Arc::new(b))
}

fn font_descriptor_bytes(doc: &Document, font_dict: &Dictionary) -> Option<Vec<u8>> {
    let desc = font_dict.get_deref(b"FontDescriptor", doc).ok()?;
    let dict = match desc {
        Object::Dictionary(d) => d,
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        _ => return None,
    };
    for key in [b"FontFile2".as_slice(), b"FontFile3", b"FontFile"] {
        let stream = dict
            .get_deref(key, doc)
            .ok()
            .and_then(|o| o.as_stream().ok())?;
        if let Ok(data) = stream.decompressed_content() {
            if FontRef::try_from_slice(&data).is_ok() || data.len() > 256 {
                return Some(data);
            }
        }
    }
    None
}

pub fn resolve_font_id(
    doc: &Document,
    resources: &Object,
    name: &[u8],
) -> Option<ObjectId> {
    let res_dict = match resources {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }?;
    let fonts = res_dict.get(b"Font").ok()?;
    let font_dict = match fonts {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }?;
    font_dict.get(name).ok()?.as_reference().ok()
}

pub struct FontCache {
    embedded: HashMap<ObjectId, Arc<Vec<u8>>>,
}

impl FontCache {
    pub fn new() -> Self {
        Self {
            embedded: HashMap::new(),
        }
    }

    pub fn get(&mut self, doc: &Document, font_id: ObjectId) -> Option<Arc<Vec<u8>>> {
        if let Some(b) = self.embedded.get(&font_id) {
            return Some(b.clone());
        }
        let dict = doc.get_dictionary(font_id).ok()?;
        let bytes = extract_font_bytes(doc, &dict)?;
        self.embedded.insert(font_id, bytes.clone());
        Some(bytes)
    }
}

pub fn draw_text_on_pixmap(
    pixmap: &mut Pixmap,
    text: &str,
    pdf_x: f32,
    pdf_y: f32,
    font_size: f32,
    color: [f32; 3],
    page_height: f32,
    viewport_scale: f32,
    font: &FontArc,
) {
    let px_scale = PxScale::from(font_size * viewport_scale);
    let mut pen_x = pdf_x * viewport_scale;
    let baseline_y = (page_height - pdf_y) * viewport_scale;

    let r = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color[2].clamp(0.0, 1.0) * 255.0) as u8;

    for ch in text.chars() {
        if ch == ' ' {
            let glyph_id = font.glyph_id(' ');
            pen_x += font.as_scaled(px_scale).h_advance(glyph_id) as f32;
            continue;
        }
        let glyph_id = font.glyph_id(ch);
        if let Some(outlined) = font.outline_glyph(glyph_id.with_scale(px_scale)) {
            let advance = font.as_scaled(px_scale).h_advance(glyph_id);
            outlined.draw(|x, y, coverage| {
                if coverage < 0.01 {
                    return;
                }
                let px = pen_x + x as f32;
                let py = baseline_y - y as f32;
                blend_pixel(pixmap, px, py, r, g, b, coverage);
            });
            pen_x += advance as f32;
        } else {
            pen_x += font_size * 0.4 * viewport_scale;
        }
    }
}

fn blend_pixel(pixmap: &mut Pixmap, x: f32, y: f32, r: u8, g: u8, b: u8, a: f32) {
    let px = x as i32;
    let py = y as i32;
    if px < 0 || py < 0 {
        return;
    }
    let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
    if px >= w || py >= h {
        return;
    }
    let idx = (py * w + px) as usize * 4;
    let data = pixmap.data_mut();
    if idx + 3 >= data.len() {
        return;
    }
    let alpha = (a * 255.0) as u8;
    let inv = 255u16.saturating_sub(alpha as u16);
    data[idx] = ((data[idx] as u16 * inv) / 255 + (r as u16 * alpha as u16) / 255) as u8;
    data[idx + 1] = ((data[idx + 1] as u16 * inv) / 255 + (g as u16 * alpha as u16) / 255) as u8;
    data[idx + 2] = ((data[idx + 2] as u16 * inv) / 255 + (b as u16 * alpha as u16) / 255) as u8;
    data[idx + 3] = 255;
}
