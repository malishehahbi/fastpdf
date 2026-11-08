use crate::types::PermissionsDto;
use lopdf::{Document, Object};

/// Default open permissions when document is not encrypted.
pub fn default_permissions() -> PermissionsDto {
    PermissionsDto {
        print: true,
        modify: true,
        copy: true,
        annotate: true,
    }
}

/// Reads user permission flags from the Encrypt dictionary when present.
pub fn extract_permissions(doc: &Document) -> PermissionsDto {
    let Some(catalog_ref) = doc.trailer.get(b"Root").ok() else {
        return default_permissions();
    };
    let Object::Reference(catalog_id) = catalog_ref else {
        return default_permissions();
    };
    let Ok(catalog) = doc.get_dictionary(*catalog_id) else {
        return default_permissions();
    };
    let Ok(encrypt_ref) = catalog.get(b"Encrypt") else {
        return default_permissions();
    };
    let Object::Reference(encrypt_id) = encrypt_ref else {
        return default_permissions();
    };
    let Ok(encrypt) = doc.get_dictionary(*encrypt_id) else {
        return default_permissions();
    };
    let p = encrypt
        .get(b"P")
        .ok()
        .and_then(|o| o.as_i64().ok())
        .unwrap_or(-4) as u32;

    // PDF permission bits (simplified; bit 3 print, 5 modify, 10 copy, 20 annotate forms)
    PermissionsDto {
        print: p & (1 << 2) == 0,
        modify: p & (1 << 4) == 0,
        copy: p & (1 << 10) == 0,
        annotate: p & (1 << 20) == 0,
    }
}

pub fn is_encrypted(doc: &Document) -> bool {
    doc.is_encrypted()
}
