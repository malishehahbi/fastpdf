use crate::types::FormFieldDto;
use lopdf::{Document, Object};

pub fn list_form_fields(doc: &Document) -> Vec<FormFieldDto> {
    let mut fields = Vec::new();
    let Some(catalog_id) = doc
        .trailer
        .get(b"Root")
        .ok()
        .and_then(|o| o.as_reference().ok())
    else {
        return fields;
    };
    let Ok(catalog) = doc.get_dictionary(catalog_id) else {
        return fields;
    };
    let Ok(acro) = catalog.get(b"AcroForm") else {
        return fields;
    };
    let acro_dict = match acro {
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        Object::Dictionary(d) => Some(d),
        _ => None,
    };
    let Some(acro_dict) = acro_dict else {
        return fields;
    };
    let field_refs = match acro_dict.get(b"Fields") {
        Ok(Object::Array(arr)) => arr
            .iter()
            .filter_map(|o| o.as_reference().ok())
            .collect::<Vec<_>>(),
        _ => return fields,
    };
    for fr in field_refs {
        collect_field(doc, fr, &mut fields);
    }
    fields
}

fn collect_field(doc: &Document, id: lopdf::ObjectId, out: &mut Vec<FormFieldDto>) {
    let Ok(dict) = doc.get_dictionary(id) else {
        return;
    };
    if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
        for kid in kids.iter() {
            if let Ok(id) = kid.as_reference() {
                collect_field(doc, id, out);
            }
        }
    }
    let name = dict
        .get(b"T")
        .ok()
        .and_then(|o| match o {
            Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
            Object::Name(b) => Some(String::from_utf8_lossy(b).into_owned()),
            _ => None,
        })
        .unwrap_or_else(|| format!("field_{id:?}"));
    let field_type = dict
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .unwrap_or_else(|| "Unknown".into());
    let value = dict
        .get(b"V")
        .ok()
        .and_then(|o| match o {
            Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
            Object::Name(b) => Some(String::from_utf8_lossy(b).into_owned()),
            Object::Boolean(b) => Some(b.to_string()),
            _ => None,
        });
    let flags = dict
        .get(b"Ff")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(0) as u32;
    let read_only = flags & (1 << 0) != 0;
    let required = flags & (1 << 1) != 0;
    out.push(FormFieldDto {
        name,
        field_type,
        value,
        flags,
        read_only,
        required,
    });
}

pub fn set_field_value(doc: &mut Document, name: &str, value: &str) -> bool {
    let fields = list_form_fields(doc);
    for meta in fields {
        if meta.name == name {
            return set_field_by_name(doc, name, value);
        }
    }
    false
}

fn set_field_by_name(doc: &mut Document, name: &str, value: &str) -> bool {
    for (_id, obj) in doc.objects.iter_mut() {
        if let Object::Dictionary(dict) = obj {
            let t = dict.get(b"T").ok().and_then(|o| match o {
                Object::String(b, _) => Some(String::from_utf8_lossy(b)),
                Object::Name(b) => Some(String::from_utf8_lossy(b)),
                _ => None,
            });
            if t.as_deref() == Some(name) {
                dict.set("V", Object::string_literal(value));
                return true;
            }
        }
    }
    false
}
