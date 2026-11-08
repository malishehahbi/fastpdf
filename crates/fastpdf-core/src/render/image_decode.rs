use lopdf::{Document, Object, Stream};

pub fn decode_image_stream(doc: &Document, stream: &Stream) -> Option<(Vec<u8>, u32, u32)> {
    let w = stream.dict.get(b"Width").ok()?.as_i64().ok()? as u32;
    let h = stream.dict.get(b"Height").ok()?.as_i64().ok()? as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let bpc = stream
        .dict
        .get(b"BitsPerComponent")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(8) as u32;
    let data = stream.decompressed_content().ok()?;

    if let Ok(img) = image::load_from_memory(&data) {
        let rgba = img.to_rgba8();
        let (iw, ih) = rgba.dimensions();
        return Some((rgba.into_raw(), iw, ih));
    }

    let cs = stream.dict.get(b"ColorSpace").ok();
    decode_by_colorspace(doc, cs, &data, w, h, bpc)
}

fn decode_by_colorspace(
    doc: &Document,
    cs: Option<&Object>,
    data: &[u8],
    w: u32,
    h: u32,
    bpc: u32,
) -> Option<(Vec<u8>, u32, u32)> {
    let resolved = match cs {
        None => None,
        Some(Object::Reference(id)) => doc.get_object(*id).ok(),
        Some(other) => Some(other),
    };
    match resolved {
        None => rgba_from_rgb_device(data, w, h, bpc),
        Some(Object::Name(n)) if is_name(n, b"DeviceRGB") || is_name(n, b"RGB") => {
            rgba_from_rgb_device(data, w, h, bpc)
        }
        Some(Object::Name(n)) if is_name(n, b"DeviceGray") || is_name(n, b"Gray") => {
            rgba_from_gray_device(data, w, h, bpc)
        }
        Some(Object::Name(n)) if is_name(n, b"DeviceCMYK") || is_name(n, b"CMYK") => {
            rgba_from_cmyk_device(data, w, h, bpc)
        }
        Some(Object::Array(arr)) if arr.first().and_then(|o| o.as_name().ok()) == Some(b"Indexed") => {
            indexed_to_rgba(doc, data, w, h, bpc, arr)
        }
        Some(Object::Array(_)) => rgba_from_rgb_device(data, w, h, bpc),
        _ => rgba_from_rgb_device(data, w, h, bpc),
    }
}

fn is_name(bytes: &[u8], name: &[u8]) -> bool {
    bytes == name
}

fn rgba_from_rgb_device(data: &[u8], w: u32, h: u32, bpc: u32) -> Option<(Vec<u8>, u32, u32)> {
    let needed = (w * h * 3 * bpc).div_ceil(8) as usize;
    if data.len() < needed {
        return None;
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    if bpc == 8 {
        for i in 0..(w * h) as usize {
            let base = i * 3;
            if base + 2 >= data.len() {
                break;
            }
            rgba.extend_from_slice(&[data[base], data[base + 1], data[base + 2], 255]);
        }
    } else {
        return None;
    }
    Some((rgba, w, h))
}

fn rgba_from_gray_device(data: &[u8], w: u32, h: u32, bpc: u32) -> Option<(Vec<u8>, u32, u32)> {
    if bpc != 8 || data.len() < (w * h) as usize {
        return None;
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for &g in data.iter().take((w * h) as usize) {
        rgba.extend_from_slice(&[g, g, g, 255]);
    }
    Some((rgba, w, h))
}

fn rgba_from_cmyk_device(data: &[u8], w: u32, h: u32, bpc: u32) -> Option<(Vec<u8>, u32, u32)> {
    if bpc != 8 || data.len() < (w * h * 4) as usize {
        return None;
    }
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for i in 0..(w * h) as usize {
        let base = i * 4;
        let c = data[base] as f32 / 255.0;
        let m = data[base + 1] as f32 / 255.0;
        let y = data[base + 2] as f32 / 255.0;
        let k = data[base + 3] as f32 / 255.0;
        let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
        let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
        let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
        rgba.extend_from_slice(&[r, g, b, 255]);
    }
    Some((rgba, w, h))
}

fn indexed_to_rgba(
    doc: &Document,
    data: &[u8],
    w: u32,
    h: u32,
    bpc: u32,
    arr: &[Object],
) -> Option<(Vec<u8>, u32, u32)> {
    if arr.len() < 4 {
        return None;
    }
    let palette = match &arr[3] {
        Object::String(bytes, _) => bytes.clone(),
        Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| match o {
            Object::String(b, _) => Some(b.clone()),
            Object::Stream(s) => s.decompressed_content().ok(),
            _ => None,
        })?,
        _ => return None,
    };
    let max_idx = palette.len().saturating_sub(1).min(255);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    if bpc == 8 {
        for &idx in data.iter().take((w * h) as usize) {
            let i = (idx as usize).min(max_idx);
            let base = i * 3;
            if base + 2 < palette.len() {
                rgba.extend_from_slice(&[palette[base], palette[base + 1], palette[base + 2], 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 255]);
            }
        }
    } else {
        return None;
    }
    Some((rgba, w, h))
}
