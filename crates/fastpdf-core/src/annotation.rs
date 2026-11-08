use crate::error::{FastPdfError, Result};
use crate::types::{AnnotationDto, Rect};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;

pub fn list_annotations(
    doc: &Document,
    pages: &BTreeMap<u32, ObjectId>,
    page_index: u32,
) -> Result<Vec<AnnotationDto>> {
    let page_id = pages
        .get(&page_index)
        .copied()
        .ok_or(FastPdfError::PageOutOfRange(page_index))?;
    let dict = doc
        .get_dictionary(page_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let annots = match dict.get(b"Annots") {
        Ok(a) => a,
        Err(_) => return Ok(vec![]),
    };
    let refs = match annots {
        Object::Array(arr) => arr
            .iter()
            .filter_map(|o| o.as_reference().ok())
            .collect::<Vec<_>>(),
        Object::Reference(r) => vec![*r],
        _ => return Ok(vec![]),
    };
    let mut out = Vec::new();
    for rid in refs {
        if let Ok(ann) = doc.get_dictionary(rid) {
            if let Some(dto) = annotation_from_dict(doc, &ann, pages) {
                out.push(dto);
            }
        }
    }
    Ok(out)
}

fn annotation_from_dict(
    doc: &Document,
    ann: &Dictionary,
    pages: &BTreeMap<u32, ObjectId>,
) -> Option<AnnotationDto> {
    let subtype = ann.get(b"Subtype").ok().and_then(|o| o.as_name().ok())?;
    let subtype = String::from_utf8_lossy(subtype).into_owned();
    let rect = ann.get(b"Rect").ok().and_then(|o| rect_object(doc, o))?;
    let contents = ann
        .get(b"Contents")
        .ok()
        .and_then(|o| match o {
            Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
            _ => None,
        });
    let flags = ann
        .get(b"F")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0) as u32;
    let mut url = None;
    let mut page_index = None;
    if subtype == "Link" {
        if let Ok(a) = ann.get(b"A") {
            if let Some(uri) = action_uri(doc, a) {
                url = Some(uri);
            }
        }
        if let Ok(dest) = ann.get(b"Dest") {
            page_index = dest_page(doc, dest, pages);
        }
    }
    Some(AnnotationDto {
        subtype,
        rect,
        contents,
        flags,
        url,
        page_index,
    })
}

pub fn hit_test(
    doc: &Document,
    pages: &BTreeMap<u32, ObjectId>,
    page_index: u32,
    x: f32,
    y: f32,
) -> Result<Vec<AnnotationDto>> {
    let all = list_annotations(doc, pages, page_index)?;
    Ok(all
        .into_iter()
        .filter(|a| {
            x >= a.rect.x
                && x <= a.rect.x + a.rect.width
                && y >= a.rect.y
                && y <= a.rect.y + a.rect.height
        })
        .collect())
}

pub fn add_highlight(
    doc: &mut Document,
    pages: &BTreeMap<u32, ObjectId>,
    page_index: u32,
    quad: Rect,
    contents: Option<&str>,
) -> Result<ObjectId> {
    let page_id = pages
        .get(&page_index)
        .copied()
        .ok_or(FastPdfError::PageOutOfRange(page_index))?;
    let annot_id = doc.new_object_id();
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"Annot".to_vec()));
    dict.set("Subtype", Object::Name(b"Highlight".to_vec()));
    dict.set(
        "Rect",
        Object::Array(vec![
            Object::Real(quad.x),
            Object::Real(quad.y),
            Object::Real(quad.x + quad.width),
            Object::Real(quad.y + quad.height),
        ]),
    );
    dict.set(
        "QuadPoints",
        Object::Array(vec![
            Object::Real(quad.x),
            Object::Real(quad.y + quad.height),
            Object::Real(quad.x + quad.width),
            Object::Real(quad.y + quad.height),
            Object::Real(quad.x),
            Object::Real(quad.y),
            Object::Real(quad.x + quad.width),
            Object::Real(quad.y),
        ]),
    );
    dict.set("C", Object::Array(vec![Object::Real(1.0), Object::Real(1.0), Object::Real(0.0)]));
    if let Some(c) = contents {
        dict.set("Contents", Object::string_literal(c));
    }
    dict.set("P", Object::Reference(page_id));
    doc.objects.insert(annot_id, Object::Dictionary(dict));

    let page_dict = doc.get_dictionary_mut(page_id).map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    match page_dict.get_mut(b"Annots") {
        Ok(Object::Array(arr)) => arr.push(Object::Reference(annot_id)),
        Ok(Object::Reference(r)) => {
            let prev = *r;
            page_dict.set(
                "Annots",
                Object::Array(vec![Object::Reference(prev), Object::Reference(annot_id)]),
            );
        }
        _ => page_dict.set("Annots", Object::Array(vec![Object::Reference(annot_id)])),
    }
    Ok(annot_id)
}

fn object_as_f32(obj: &Object) -> f32 {
    match obj {
        Object::Real(v) => *v,
        Object::Integer(v) => *v as f32,
        _ => 0.0,
    }
}

fn rect_object(doc: &Document, obj: &Object) -> Option<Rect> {
    match obj {
        Object::Array(arr) if arr.len() >= 4 => {
            let v: Vec<f32> = arr.iter().map(object_as_f32).collect();
            Some(Rect {
                x: v[0],
                y: v[1],
                width: v[2] - v[0],
                height: v[3] - v[1],
            })
        }
        Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| rect_object(doc, o)),
        _ => None,
    }
}

fn action_uri(doc: &Document, action: &Object) -> Option<String> {
    let dict = match action {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    }?;
    let uri = dict.get(b"URI").ok()?;
    match uri {
        Object::String(b, _) | Object::Name(b) => Some(String::from_utf8_lossy(b).into_owned()),
        _ => None,
    }
}

fn dest_page(doc: &Document, dest: &Object, pages: &BTreeMap<u32, ObjectId>) -> Option<u32> {
    let page_ref = match dest {
        Object::Array(arr) if !arr.is_empty() => arr[0].as_reference().ok(),
        Object::Reference(r) => Some(*r),
        _ => None,
    }?;
    pages
        .iter()
        .find(|(_, id)| **id == page_ref)
        .map(|(idx, _)| *idx)
}
