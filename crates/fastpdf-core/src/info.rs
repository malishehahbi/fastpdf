use crate::types::DocumentInfo;
use lopdf::{Document, Object};

pub fn extract_info(doc: &Document) -> DocumentInfo {
    let trailer = doc.trailer.get(b"Info").ok();
    let mut info = DocumentInfo {
        title: None,
        author: None,
        subject: None,
        creator: None,
        producer: None,
        creation_date: None,
        modification_date: None,
    };
    let Some(Object::Reference(id)) = trailer else {
        return info;
    };
    let Ok(dict) = doc.get_dictionary(*id) else {
        return info;
    };
    info.title = dict.get(b"Title").ok().and_then(string_obj);
    info.author = dict.get(b"Author").ok().and_then(string_obj);
    info.subject = dict.get(b"Subject").ok().and_then(string_obj);
    info.creator = dict.get(b"Creator").ok().and_then(string_obj);
    info.producer = dict.get(b"Producer").ok().and_then(string_obj);
    info.creation_date = dict.get(b"CreationDate").ok().and_then(string_obj);
    info.modification_date = dict.get(b"ModDate").ok().and_then(string_obj);
    info
}

fn string_obj(obj: &Object) -> Option<String> {
    crate::pdf_num::decode_object_text_string(obj)
}
