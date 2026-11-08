use crate::error::{FastPdfError, Result};
use crate::types::TextSpan;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct TextState {
    tm: [f32; 6],
    font_size: f32,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            tm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            font_size: 12.0,
        }
    }
}

pub fn extract_page_text(doc: &Document, page_id: ObjectId) -> Result<String> {
    let spans = extract_text_spans(doc, page_id)?;
    Ok(spans.into_iter().map(|s| s.text).collect::<Vec<_>>().join(" "))
}

pub fn extract_text_spans(doc: &Document, page_id: ObjectId) -> Result<Vec<TextSpan>> {
    let bytes = doc
        .get_page_content(page_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let content = Content::decode(&bytes).map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let mut spans = Vec::new();
    let mut text_state = TextState::default();
    let mut ctm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut stack: Vec<[f32; 6]> = Vec::new();

    for Operation { operator, operands } in content.operations {
        match operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            "cm" if operands.len() >= 6 => {
                let m = op_matrix(&operands);
                ctm = mul_matrix(ctm, m);
            }
            "Tm" if operands.len() >= 6 => {
                text_state.tm = op_matrix(&operands);
            }
            "Tf" if operands.len() >= 2 => {
                text_state.font_size = operands[1].as_f32().unwrap_or(12.0);
            }
            "Td" | "TD" if operands.len() >= 2 => {
                let tx = operands[0].as_f32().unwrap_or(0.0);
                let ty = operands[1].as_f32().unwrap_or(0.0);
                text_state.tm = mul_matrix(text_state.tm, [1.0, 0.0, 0.0, 1.0, tx, ty]);
            }
            "Tj" if !operands.is_empty() => {
                if let Some(s) = operand_string(&operands[0]) {
                    push_span(&mut spans, &s, &text_state, ctm);
                }
            }
            "TJ" if !operands.is_empty() => {
                if let Object::Array(arr) = &operands[0] {
                    for item in arr {
                        if let Some(s) = operand_string(item) {
                            push_span(&mut spans, &s, &text_state, ctm);
                        }
                    }
                }
            }
            "'" if !operands.is_empty() => {
                if let Some(s) = operand_string(&operands[0]) {
                    push_span(&mut spans, &s, &text_state, ctm);
                }
            }
            "\"" if operands.len() >= 3 => {
                if let Some(s) = operand_string(&operands[2]) {
                    push_span(&mut spans, &s, &text_state, ctm);
                }
            }
            _ => {}
        }
    }
    Ok(spans)
}

pub fn all_pages_text(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> Result<Vec<(u32, String)>> {
    let mut out = Vec::new();
    for (idx, id) in pages {
        let text = extract_page_text(doc, *id)?;
        out.push((*idx, text));
    }
    Ok(out)
}

fn push_span(spans: &mut Vec<TextSpan>, text: &str, ts: &TextState, ctm: [f32; 6]) {
    if text.is_empty() {
        return;
    }
    let m = mul_matrix(ctm, ts.tm);
    let x = m[4];
    let y = m[5];
    let width = text.len() as f32 * ts.font_size * 0.5;
    spans.push(TextSpan {
        text: text.to_string(),
        x,
        y,
        width,
        height: ts.font_size,
    });
}

fn operand_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(pdf_string_decode(bytes)),
        _ => None,
    }
}

fn pdf_string_decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn op_matrix(ops: &[Object]) -> [f32; 6] {
    [
        ops[0].as_f32().unwrap_or(1.0),
        ops[1].as_f32().unwrap_or(0.0),
        ops[2].as_f32().unwrap_or(0.0),
        ops[3].as_f32().unwrap_or(1.0),
        ops[4].as_f32().unwrap_or(0.0),
        ops[5].as_f32().unwrap_or(0.0),
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
