use crate::error::{FastPdfError, Result};
use crate::pdf_num::{cmyk_to_rgb, decode_pdf_text_string, object_as_f32};
use crate::render::fonts::{FontCache, resolve_font_id};
use crate::render::image_decode::decode_image_stream;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};

const MAX_FORM_DEPTH: u32 = 24;

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
        font_embed: Option<std::sync::Arc<Vec<u8>>>,
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
    font_id: Option<ObjectId>,
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
            font_id: None,
        }
    }
}

enum XObjectResolved {
    Image(Vec<u8>, u32, u32),
    Form {
        operations: Vec<Operation>,
        resources: Option<Object>,
        matrix: [f32; 6],
    },
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
    let mut font_cache = FontCache::new();
    process_operations(
        doc,
        &content.operations,
        resources.as_ref(),
        &mut state,
        &mut stack,
        &mut items,
        &mut font_cache,
        0,
    )?;
    Ok(items)
}

fn process_operations(
    doc: &Document,
    operations: &[Operation],
    resources: Option<&Object>,
    state: &mut GfxState,
    stack: &mut Vec<GfxState>,
    items: &mut Vec<DisplayItem>,
    font_cache: &mut FontCache,
    depth: u32,
) -> Result<()> {
    for Operation { operator, operands } in operations {
        match operator.as_str() {
            "q" => stack.push(state.clone()),
            "Q" => {
                if let Some(s) = stack.pop() {
                    *state = s;
                }
            }
            "cm" if operands.len() >= 6 => {
                let m = matrix_from_ops(operands);
                state.ctm = mul_matrix(state.ctm, m);
            }
            "w" if !operands.is_empty() => {
                state.line_width = object_as_f32(&operands[0]).max(0.01);
            }
            "RG" if operands.len() >= 3 => {
                state.stroke = rgb_from_ops(operands);
            }
            "rg" if operands.len() >= 3 => {
                state.fill = rgb_from_ops(operands);
            }
            "K" if operands.len() >= 4 => {
                state.stroke = cmyk_from_ops(operands);
            }
            "k" if operands.len() >= 4 => {
                state.fill = cmyk_from_ops(operands);
            }
            "G" if !operands.is_empty() => {
                let g = object_as_f32(&operands[0]);
                state.stroke = [g, g, g];
            }
            "g" if !operands.is_empty() => {
                let g = object_as_f32(&operands[0]);
                state.fill = [g, g, g];
            }
            "m" if operands.len() >= 2 => {
                let x = object_as_f32(&operands[0]);
                let y = object_as_f32(&operands[1]);
                state.path.clear();
                state.path.push((x, y));
            }
            "l" if operands.len() >= 2 => {
                let x = object_as_f32(&operands[0]);
                let y = object_as_f32(&operands[1]);
                state.path.push((x, y));
            }
            "c" if operands.len() >= 6 => {
                let x = object_as_f32(&operands[4]);
                let y = object_as_f32(&operands[5]);
                state.path.push((x, y));
            }
            "v" | "y" if operands.len() >= 4 => {
                let x = object_as_f32(&operands[2]);
                let y = object_as_f32(&operands[3]);
                state.path.push((x, y));
            }
            "re" if operands.len() >= 4 => {
                let x = object_as_f32(&operands[0]);
                let y = object_as_f32(&operands[1]);
                let w = object_as_f32(&operands[2]);
                let h = object_as_f32(&operands[3]);
                state.path = vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
            }
            "h" => {
                if let (Some(first), Some(last)) = (state.path.first(), state.path.last()) {
                    if first != last {
                        state.path.push(*first);
                    }
                }
            }
            "n" => state.path.clear(),
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
                let Some(res) = resources else { continue };
                let Some(xo) = resolve_xobject(doc, res, &name) else {
                    continue;
                };
                match xo {
                    XObjectResolved::Image(pixels, width, height) => {
                        items.push(DisplayItem::Image {
                            pixels,
                            width,
                            height,
                            ctm: state.ctm,
                        });
                    }
                    XObjectResolved::Form {
                        operations,
                        resources: form_res,
                        matrix,
                    } if depth < MAX_FORM_DEPTH => {
                        let saved = state.ctm;
                        state.ctm = mul_matrix(state.ctm, matrix);
                        let merged = form_res.as_ref().or(Some(res));
                        process_operations(
                            doc,
                            &operations,
                            merged,
                            state,
                            stack,
                            items,
                            font_cache,
                            depth + 1,
                        )?;
                        state.ctm = saved;
                    }
                    XObjectResolved::Form { .. } => {}
                }
            }
            "BT" => state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            "ET" => {}
            "Tm" if operands.len() >= 6 => {
                state.text_matrix = matrix_from_ops(operands);
            }
            "Tf" if operands.len() >= 2 => {
                state.font_size = object_as_f32(&operands[1]).max(1.0);
                let font_name = match &operands[0] {
                    Object::Name(n) => n.as_slice(),
                    _ => &[],
                };
                if let Some(res) = resources {
                    state.font_id = resolve_font_id(doc, res, font_name);
                }
            }
            "Td" | "TD" if operands.len() >= 2 => {
                let tx = object_as_f32(&operands[0]);
                let ty = object_as_f32(&operands[1]);
                state.text_matrix = mul_matrix(state.text_matrix, [1.0, 0.0, 0.0, 1.0, tx, ty]);
            }
            "Tj" if !operands.is_empty() => {
                if let Some(text) = decode_text_operand(doc, state.font_id, &operands[0]) {
                    push_text(doc, items, &text, state, font_cache);
                }
            }
            "TJ" if !operands.is_empty() => {
                if let Object::Array(arr) = &operands[0] {
                    for item in arr {
                        if let Some(text) = decode_text_operand(doc, state.font_id, item) {
                            push_text(doc, items, &text, state, font_cache);
                        }
                    }
                }
            }
            "'" if !operands.is_empty() => {
                if let Some(text) = decode_text_operand(doc, state.font_id, &operands[0]) {
                    push_text(doc, items, &text, state, font_cache);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn decode_text_operand(doc: &Document, font_id: Option<ObjectId>, obj: &Object) -> Option<String> {
    let bytes = match obj {
        Object::String(b, _) => b.clone(),
        _ => return None,
    };
    if let Some(fid) = font_id {
        if let Ok(dict) = doc.get_dictionary(fid) {
            if let Ok(enc) = dict.get_font_encoding(doc) {
                if let Ok(s) = enc.bytes_to_string(&bytes) {
                    return Some(s);
                }
            }
        }
    }
    Some(decode_pdf_text_string(&bytes))
}

fn push_text(
    doc: &Document,
    items: &mut Vec<DisplayItem>,
    text: &str,
    state: &GfxState,
    font_cache: &mut FontCache,
) {
    if text.is_empty() {
        return;
    }
    let font_embed = state
        .font_id
        .and_then(|id| font_cache.get(doc, id));
    let tm = mul_matrix(state.ctm, state.text_matrix);
    items.push(DisplayItem::Text {
        text: text.to_string(),
        ctm: tm,
        size: state.font_size,
        color: state.fill,
        font_embed,
    });
}

fn resolve_xobject(doc: &Document, resources: &Object, name: &[u8]) -> Option<XObjectResolved> {
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
    let subtype = stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())?;

    if subtype == b"Image" {
        if let Some((pixels, width, height)) = decode_image_stream(doc, stream) {
            return Some(XObjectResolved::Image(pixels, width, height));
        }
        return None;
    }

    if subtype == b"Form" {
        let data = stream.decompressed_content().ok()?;
        let content = Content::decode(&data).ok()?;
        let matrix = stream
            .dict
            .get(b"Matrix")
            .ok()
            .and_then(|o| match o {
                Object::Array(arr) if arr.len() >= 6 => Some(matrix_from_ops(arr)),
                _ => None,
            })
            .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let form_resources = stream.dict.get(b"Resources").ok().cloned();
        return Some(XObjectResolved::Form {
            operations: content.operations,
            resources: form_resources,
            matrix,
        });
    }
    None
}

fn transform_path(path: &[(f32, f32)], m: [f32; 6]) -> Vec<(f32, f32)> {
    path.iter()
        .map(|&(x, y)| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]))
        .collect()
}

fn matrix_from_ops(ops: &[Object]) -> [f32; 6] {
    [
        object_as_f32(&ops[0]),
        object_as_f32(&ops[1]),
        object_as_f32(&ops[2]),
        object_as_f32(&ops[3]),
        object_as_f32(&ops[4]),
        object_as_f32(&ops[5]),
    ]
}

fn rgb_from_ops(ops: &[Object]) -> [f32; 3] {
    [
        object_as_f32(&ops[0]),
        object_as_f32(&ops[1]),
        object_as_f32(&ops[2]),
    ]
}

fn cmyk_from_ops(ops: &[Object]) -> [f32; 3] {
    cmyk_to_rgb(
        object_as_f32(&ops[0]),
        object_as_f32(&ops[1]),
        object_as_f32(&ops[2]),
        object_as_f32(&ops[3]),
    )
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

