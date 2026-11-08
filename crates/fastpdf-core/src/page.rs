use crate::error::{FastPdfError, Result};
use crate::types::{PageInfo, Rect};
use lopdf::{Document, Object, ObjectId};
use std::collections::BTreeMap;

pub fn page_count(pages: &BTreeMap<u32, ObjectId>) -> u32 {
    pages.len() as u32
}

pub fn page_id(pages: &BTreeMap<u32, ObjectId>, index: u32) -> Result<ObjectId> {
    pages
        .get(&index)
        .copied()
        .ok_or(FastPdfError::PageOutOfRange(index))
}

pub fn page_info(doc: &Document, pages: &BTreeMap<u32, ObjectId>, index: u32) -> Result<PageInfo> {
    let id = page_id(pages, index)?;
    let dict = doc
        .get_dictionary(id)
        .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
    let (w, h) = media_box_size(doc, &dict)?;
    let rotation = dict
        .get(b"Rotate")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0) as u32;
    let label = None; // filled by document via page labels pass
    Ok(PageInfo {
        index,
        width: w,
        height: h,
        rotation: rotation % 360,
        label,
    })
}

pub fn media_box_size(doc: &Document, page_dict: &lopdf::Dictionary) -> Result<(f32, f32)> {
    let box_obj = page_dict
        .get(b"MediaBox")
        .or_else(|_| page_dict.get(b"CropBox"))
        .map_err(|_| FastPdfError::CorruptPdf("page missing MediaBox".into()))?;
    rect_from_object(doc, box_obj).map(|r| (r.width, r.height))
}

pub fn page_rect(doc: &Document, page_dict: &lopdf::Dictionary) -> Result<Rect> {
    let box_obj = page_dict
        .get(b"MediaBox")
        .or_else(|_| page_dict.get(b"CropBox"))
        .map_err(|_| FastPdfError::CorruptPdf("page missing MediaBox".into()))?;
    rect_from_object(doc, box_obj)
}

fn object_as_f32(obj: &Object) -> f32 {
    match obj {
        Object::Real(v) => *v,
        Object::Integer(v) => *v as f32,
        _ => 0.0,
    }
}

fn rect_from_object(doc: &Document, obj: &Object) -> Result<Rect> {
    match obj {
        Object::Array(arr) if arr.len() >= 4 => {
            let vals: Vec<f32> = arr.iter().take(4).map(object_as_f32).collect();
            Ok(Rect {
                x: vals[0],
                y: vals[1],
                width: vals[2] - vals[0],
                height: vals[3] - vals[1],
            })
        }
        Object::Reference(id) => {
            let o = doc
                .get_object(*id)
                .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
            rect_from_object(doc, o)
        }
        _ => Err(FastPdfError::CorruptPdf("invalid rect".into())),
    }
}

pub fn resolve_page_labels(doc: &Document, pages: &BTreeMap<u32, ObjectId>) -> BTreeMap<u32, String> {
    let mut labels = BTreeMap::new();
    let Some(catalog_id) = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
    else {
        return labels;
    };
    let Ok(catalog) = doc.get_dictionary(catalog_id) else {
        return labels;
    };
    let Ok(Object::Reference(pl_ref)) = catalog.get(b"PageLabels") else {
        return labels;
    };
    let Ok(pl) = doc.get_dictionary(*pl_ref) else {
        return labels;
    };
    let nums = match pl.get(b"Nums").ok() {
        Some(Object::Array(a)) => a.clone(),
        _ => return labels,
    };
    let mut i = 0usize;
    while i + 1 < nums.len() {
        let start = nums[i].as_i64().unwrap_or(0) as u32;
        if let Object::Dictionary(spec) = &nums[i + 1] {
            let prefix = spec
                .get(b"P")
                .ok()
                .and_then(|o| match o {
                    Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
                    _ => None,
                })
                .unwrap_or_default();
            let style = spec.get(b"S").ok().and_then(|o| o.as_name().ok());
            let mut n = spec.get(b"St").ok().and_then(|o| o.as_i64().ok()).unwrap_or(1) as u32;
            for (idx, _) in pages.iter().skip(start as usize) {
                let label = match style {
                    Some(b"D") => format!("{n}"),
                    Some(b"R") => roman(n),
                    Some(b"r") => roman(n).to_lowercase(),
                    Some(b"A") => alpha(n, true),
                    Some(b"a") => alpha(n, false),
                    _ => format!("{prefix}{n}"),
                };
                labels.insert(*idx, if prefix.is_empty() { label } else { format!("{prefix}{label}") });
                n += 1;
            }
        }
        i += 2;
    }
    labels
}

fn roman(mut n: u32) -> String {
    const PAIRS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in PAIRS {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn alpha(mut n: u32, upper: bool) -> String {
    let mut out = String::new();
    while n > 0 {
        n -= 1;
        let c = (b'A' + (n % 26) as u8) as char;
        out.insert(0, if upper { c } else { c.to_ascii_lowercase() });
        n /= 26;
    }
    out
}
