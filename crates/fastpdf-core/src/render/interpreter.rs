use crate::error::{FastPdfError, Result};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};
#[derive(Debug, Clone)]
pub enum DisplayItem {
    FillPath(Vec<(f32, f32)>, [f32; 3]),
    StrokePath(Vec<(f32, f32)>, [f32; 3], f32),
    Image {
        pixels: Vec<u8>,
        width: u32,
        height: u32,
        ctm: [f32; 6],
    },
    Text {
        text: String,
        ctm: [f32; 6],
        size: f32,
        color: [f32; 3],
    },
}

#[derive(Clone)]
struct GfxState {
    ctm: [f32; 6],
    stroke: [f32; 3],
    fill: [f32; 3],
    line_width: f32,
    path: Vec<(f32, f32)>,
    text_matrix: [f32; 6],
    font_size: f32,
}

impl Default for GfxState {
    fn default() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            stroke: [0.0, 0.0, 0.0],
            fill: [0.0, 0.0, 0.0],
            line_width: 1.0,
            path: Vec::new(),
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            font_size: 12.0,
        }
    }
}

pub fn interpret_page(
    doc: &Document,
    page_id: ObjectId,
    _page_width: f32,
    _page_height: f32,
) -> Result<Vec<DisplayItem>> {
    let page_dict = doc
        .get_dictionary(page_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let resources = page_dict.get(b"Resources").ok().cloned();
    let bytes = doc
        .get_page_content(page_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let content = Content::decode(&bytes).map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let mut state = GfxState::default();
    let mut stack: Vec<GfxState> = Vec::new();
    let mut items = Vec::new();

    for Operation { operator, operands } in content.operations {
        match operator.as_str() {
            "q" => stack.push(state.clone()),
            "Q" => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
            }
            "cm" if operands.len() >= 6 => {
                let m = matrix_from_ops(&operands);
                state.ctm = mul_matrix(state.ctm, m);
            }
            "w" if !operands.is_empty() => {
                state.line_width = operands[0].as_f32().unwrap_or(1.0);
            }
            "RG" if operands.len() >= 3 => {
                state.stroke = rgb_from_ops(&operands);
            }
            "rg" if operands.len() >= 3 => {
                state.fill = rgb_from_ops(&operands);
            }
            "G" if !operands.is_empty() => {
                let g = operands[0].as_f32().unwrap_or(0.0);
                state.stroke = [g, g, g];
            }
            "g" if !operands.is_empty() => {
                let g = operands[0].as_f32().unwrap_or(0.0);
                state.fill = [g, g, g];
            }
            "m" if operands.len() >= 2 => {
                let x = operands[0].as_f32().unwrap_or(0.0);
                let y = operands[1].as_f32().unwrap_or(0.0);
                state.path.clear();
                state.path.push((x, y));
            }
            "l" if operands.len() >= 2 => {
                let x = operands[0].as_f32().unwrap_or(0.0);
                let y = operands[1].as_f32().unwrap_or(0.0);
                state.path.push((x, y));
            }
            "re" if operands.len() >= 4 => {
                let x = operands[0].as_f32().unwrap_or(0.0);
                let y = operands[1].as_f32().unwrap_or(0.0);
                let w = operands[2].as_f32().unwrap_or(0.0);
                let h = operands[3].as_f32().unwrap_or(0.0);
                state.path = vec![
                    (x, y),
                    (x + w, y),
                    (x + w, y + h),
                    (x, y + h),
                ];
            }
            "h" => {
                if let (Some(first), Some(last)) = (state.path.first(), state.path.last()) {
                    if first != last {
                        state.path.push(*first);
                    }
                }
            }
            "S" | "s" => {
                if state.path.len() >= 2 {
                    let path = transform_path(&state.path, state.ctm);
                    items.push(DisplayItem::StrokePath(path, state.stroke, state.line_width));
                }
                state.path.clear();
            }
            "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" => {
                if state.path.len() >= 3 {
                    let path = transform_path(&state.path, state.ctm);
                    items.push(DisplayItem::FillPath(path, state.fill));
                }
                if matches!(operator.as_str(), "B" | "B*" | "b" | "b*") && state.path.len() >= 2 {
                    let path = transform_path(&state.path, state.ctm);
                    items.push(DisplayItem::StrokePath(path, state.stroke, state.line_width));
                }
                state.path.clear();
            }
            "Do" if !operands.is_empty() => {
                let name = match &operands[0] {
                    Object::Name(n) => n.clone(),
                    _ => continue,
                };
                if let Some(res) = &resources {
                    if let Some(img) = resolve_image(doc, res, &name) {
                        items.push(DisplayItem::Image {
                            pixels: img.0,
                            width: img.1,
                            height: img.2,
                            ctm: state.ctm,
                        });
                    }
                }
            }
            "BT" => {
                state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "ET" => {}
            "Tm" if operands.len() >= 6 => {
                state.text_matrix = matrix_from_ops(&operands);
            }
            "Tf" if operands.len() >= 2 => {
                state.font_size = operands[1].as_f32().unwrap_or(12.0);
            }
            "Td" | "TD" if operands.len() >= 2 => {
                let tx = operands[0].as_f32().unwrap_or(0.0);
                let ty = operands[1].as_f32().unwrap_or(0.0);
                state.text_matrix = mul_matrix(state.text_matrix, [1.0, 0.0, 0.0, 1.0, tx, ty]);
            }
            "Tj" if !operands.is_empty() => {
                if let Some(text) = operand_string(&operands[0]) {
                    let tm = mul_matrix(state.ctm, state.text_matrix);
                    items.push(DisplayItem::Text {
                        text,
                        ctm: tm,
                        size: state.font_size,
                        color: state.fill,
                    });
                }
            }
            "TJ" if !operands.is_empty() => {
                if let Object::Array(arr) = &operands[0] {
                    for item in arr {
                        if let Some(text) = operand_string(item) {
                            let tm = mul_matrix(state.ctm, state.text_matrix);
                            items.push(DisplayItem::Text {
                                text,
                                ctm: tm,
                                size: state.font_size,
                                color: state.fill,
                            });
                        }
                    }
                }
            }
            "'" if !operands.is_empty() => {
                if let Some(text) = operand_string(&operands[0]) {
                    let tm = mul_matrix(state.ctm, state.text_matrix);
                    items.push(DisplayItem::Text {
                        text,
                        ctm: tm,
                        size: state.font_size,
                        color: state.fill,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(items)
}

fn resolve_image(
    doc: &Document,
    resources: &Object,
    name: &[u8],
) -> Option<(Vec<u8>, u32, u32)> {
    let res_dict = match resources {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }?;
    let xobjects = res_dict.get(b"XObject").ok()?;
    let xo_dict = match xobjects {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }?;
    let xobj_ref = xo_dict.get(name).ok()?.as_reference().ok()?;
    let stream = doc.get_object(xobj_ref).ok()?;
    let Object::Stream(stream) = stream else {
        return None;
    };
    let is_image = stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) == Some(b"Image");
    if !is_image {
        return None;
    }
    let w = stream.dict.get(b"Width").ok()?.as_i64().ok()? as u32;
    let h = stream.dict.get(b"Height").ok()?.as_i64().ok()? as u32;
    let data = stream.decompressed_content().ok()?;
    if let Ok(img) = image::load_from_memory(&data) {
        let rgba = img.to_rgba8();
        let (iw, ih) = rgba.dimensions();
        return Some((rgba.into_raw(), iw, ih));
    }
    // Raw RGB
    if data.len() >= (w * h * 3) as usize {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for i in 0..(w * h) as usize {
            let base = i * 3;
            rgba.extend_from_slice(&[data[base], data[base + 1], data[base + 2], 255]);
        }
        return Some((rgba, w, h));
    }
    None
}

fn transform_path(path: &[(f32, f32)], m: [f32; 6]) -> Vec<(f32, f32)> {
    path.iter()
        .map(|&(x, y)| {
            (
                m[0] * x + m[2] * y + m[4],
                m[1] * x + m[3] * y + m[5],
            )
        })
        .collect()
}

fn matrix_from_ops(ops: &[Object]) -> [f32; 6] {
    [
        ops[0].as_f32().unwrap_or(1.0),
        ops[1].as_f32().unwrap_or(0.0),
        ops[2].as_f32().unwrap_or(0.0),
        ops[3].as_f32().unwrap_or(1.0),
        ops[4].as_f32().unwrap_or(0.0),
        ops[5].as_f32().unwrap_or(0.0),
    ]
}

fn rgb_from_ops(ops: &[Object]) -> [f32; 3] {
    [
        ops[0].as_f32().unwrap_or(0.0),
        ops[1].as_f32().unwrap_or(0.0),
        ops[2].as_f32().unwrap_or(0.0),
    ]
}

fn mul_matrix(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn operand_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
        _ => None,
    }
}
