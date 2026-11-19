use lopdf::decode_text_string;
use lopdf::{Object, StringFormat};

#[inline]
pub fn object_as_f32(obj: &Object) -> f32 {
    match obj {
        Object::Real(v) => *v,
        Object::Integer(v) => *v as f32,
        _ => 0.0,
    }
}

/// Decode a PDF **text string** (bookmarks, info, annotations) per ISO 32000 + common producer quirks.
pub fn decode_pdf_text_string(bytes: &[u8]) -> String {
    if let [0xFF, 0xFE, rest @ ..] = bytes {
        return trim_bom(utf16_le(rest));
    }
    if !bytes.is_empty() && !bytes.is_ascii() {
        if let Ok(utf8) = std::str::from_utf8(bytes) {
            return trim_bom(utf8.to_owned());
        }
    }
    let obj = Object::String(bytes.to_vec(), StringFormat::Literal);
    decode_text_string(&obj)
        .map(trim_bom)
        .unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
}

pub fn decode_object_text_string(obj: &Object) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(decode_pdf_text_string(bytes)),
        Object::Name(name) => Some(String::from_utf8_lossy(name).into_owned()),
        _ => None,
    }
}

fn trim_bom(s: String) -> String {
    s.trim_start_matches('\u{FEFF}').to_owned()
}

fn utf16_le(rest: &[u8]) -> String {
    let units: Vec<u16> = rest
        .chunks(2)
        .map(|c| {
            if c.len() == 2 {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_le_bytes([c[0], 0])
            }
        })
        .collect();
    String::from_utf16_lossy(&units)
}

#[allow(dead_code)]
pub fn cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> [f32; 3] {
    [
        (1.0 - c) * (1.0 - k),
        (1.0 - m) * (1.0 - k),
        (1.0 - y) * (1.0 - k),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_outline_like_pdfcraft() {
        assert_eq!(
            decode_pdf_text_string(&[0xFE, 0xFF, 0x7B, 0x2C, 0x4E, 0x00, 0x7A, 0xE0]),
            "第一章"
        );
        assert_eq!(
            decode_pdf_text_string(&[0xEF, 0xBB, 0xBF, 0xE7, 0x9B, 0xAE, 0xE5, 0xBD, 0x95]),
            "目录"
        );
        assert_eq!(decode_pdf_text_string(b"\xE6\xA6\x82\xE8\xBF\xB0"), "概述");
        assert_eq!(decode_pdf_text_string(&[0x43, 0x61, 0x66, 0xE9]), "Café");
    }
}
