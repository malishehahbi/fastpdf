use crate::error::{FastPdfError, Result};
use crate::pdf_num::decode_object_text_string;
use crate::types::OutlineItem;
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;

pub fn build_outline(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> Result<Vec<OutlineItem>> {
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| FastPdfError::CorruptPdf("missing catalog".into()))?;
    let catalog = doc
        .get_dictionary(catalog_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let outlines_ref = match catalog.get(b"Outlines") {
        Ok(r) => r.as_reference().ok(),
        Err(_) => None,
    };
    let Some(outlines_id) = outlines_ref else {
        return Ok(vec![]);
    };
    let outlines = doc
        .get_dictionary(outlines_id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let first = outlines
        .get(b"First")
        .ok()
        .and_then(|o| o.as_reference().ok());
    let Some(first_id) = first else {
        return Ok(vec![]);
    };
    let mut items = Vec::new();
    let mut current = Some(first_id);
    while let Some(id) = current {
        if let Ok(item) = doc.get_dictionary(id) {
            let title = item
                .get(b"Title")
                .ok()
                .and_then(decode_object_text_string)
                .unwrap_or_default();
            let page_index = resolve_dest_page(doc, &item, pages);
            let mut children = Vec::new();
            if let Ok(Object::Reference(first_child)) = item.get(b"First") {
                children = collect_outline_children(doc, *first_child, pages)?;
            }
            items.push(OutlineItem {
                title,
                page_index,
                children,
            });
        }
        current = doc
            .get_dictionary(id)
            .ok()
            .and_then(|d| d.get(b"Next").ok())
            .and_then(|o| o.as_reference().ok());
    }
    Ok(items)
}

fn collect_outline_children(
    doc: &Document,
    first_id: ObjectId,
    pages: &BTreeMap<u32, ObjectId>,
) -> Result<Vec<OutlineItem>> {
    let mut items = Vec::new();
    let mut current = Some(first_id);
    while let Some(id) = current {
        if let Ok(item) = doc.get_dictionary(id) {
            let title = item
                .get(b"Title")
                .ok()
                .and_then(decode_object_text_string)
                .unwrap_or_default();
            let page_index = resolve_dest_page(doc, &item, pages);
            let mut children = Vec::new();
            if let Ok(Object::Reference(first_child)) = item.get(b"First") {
                children = collect_outline_children(doc, *first_child, pages)?;
            }
            items.push(OutlineItem {
                title,
                page_index,
                children,
            });
        }
        current = doc
            .get_dictionary(id)
            .ok()
            .and_then(|d| d.get(b"Next").ok())
            .and_then(|o| o.as_reference().ok());
    }
    Ok(items)
}

fn resolve_dest_page(
    doc: &Document,
    item: &lopdf::Dictionary,
    pages: &BTreeMap<u32, ObjectId>,
) -> Option<u32> {
    if let Ok(dest) = item.get(b"Dest") {
        return dest_to_page_index(doc, dest, pages);
    }
    if let Ok(action) = item.get(b"A") {
        if let Object::Reference(aid) = action {
            if let Ok(dict) = doc.get_dictionary(*aid) {
                if let Ok(dest) = dict.get(b"D") {
                    return dest_to_page_index(doc, dest, pages);
                }
            }
        } else if let Object::Dictionary(dict) = action {
            if let Ok(dest) = dict.get(b"D") {
                return dest_to_page_index(doc, dest, pages);
            }
        }
    }
    None
}

fn dest_to_page_index(
    doc: &Document,
    dest: &Object,
    pages: &BTreeMap<u32, ObjectId>,
) -> Option<u32> {
    let page_ref = match dest {
        Object::Array(arr) if !arr.is_empty() => arr[0].as_reference().ok(),
        Object::Reference(r) => Some(*r),
        Object::Name(_) => None,
        _ => None,
    }?;
    pages
        .iter()
        .find(|(_, id)| **id == page_ref)
        .map(|(idx, _)| *idx)
}
