//! FastPDF core — pure Rust PDF engine.

mod annotation;
mod cache;
mod document;
mod error;
mod pdf_num;
mod form;
mod info;
mod outline;
mod page;
mod render;
mod search;
mod security;
mod text;
mod text_layout;
mod types;

pub use document::FastPdfDocument;
pub use error::{FastPdfError, Result};
pub use types::*;

/// Engine version string (semver aligned with workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Document, Object, Stream};

    fn minimal_pdf_with_outline() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();
        let outline_id = doc.new_object_id();
        let item_id = doc.new_object_id();
        let catalog_id = doc.new_object_id();

        let content = Stream::new(
            dictionary! {
                "Length" => Object::Integer(51),
            },
            b"BT /F1 24 Tf 100 700 Td (Hello FastPDF) Tj ET".to_vec(),
        );
        let content_id = doc.add_object(Object::Stream(content));

        doc.objects.insert(
            page_id,
            Object::Dictionary(dictionary! {
                "Type" => "Page",
                "Parent" => Object::Reference(pages_id),
                "MediaBox" => Object::Array(vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(612.0),
                    Object::Real(792.0),
                ]),
                "Resources" => Object::Dictionary(dictionary! {
                    "Font" => Object::Dictionary(dictionary! {
                        "F1" => Object::Dictionary(dictionary! {
                            "Type" => Object::Name(b"Font".to_vec()),
                            "Subtype" => Object::Name(b"Type1".to_vec()),
                            "BaseFont" => Object::Name(b"Helvetica".to_vec()),
                        }),
                    }),
                }),
                "Contents" => Object::Reference(content_id),
            }),
        );
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Count" => 1,
                "Kids" => vec![Object::Reference(page_id)],
            }),
        );
        doc.objects.insert(
            item_id,
            Object::Dictionary(dictionary! {
                "Title" => Object::string_literal("Chapter 1"),
                "Dest" => vec![Object::Reference(page_id), Object::Name(b"Fit".to_vec())],
            }),
        );
        doc.objects.insert(
            outline_id,
            Object::Dictionary(dictionary! {
                "Type" => "Outlines",
                "First" => Object::Reference(item_id),
                "Last" => Object::Reference(item_id),
            }),
        );
        doc.objects.insert(
            catalog_id,
            Object::Dictionary(dictionary! {
                "Type" => "Catalog",
                "Pages" => Object::Reference(pages_id),
                "Outlines" => Object::Reference(outline_id),
            }),
        );
        doc.trailer.set("Root", Object::Reference(catalog_id));
        let info_id = doc.add_object(dictionary! {
            "Title" => Object::string_literal("Test"),
        });
        doc.trailer.set("Info", Object::Reference(info_id));
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn open_outline_and_render() {
        let bytes = minimal_pdf_with_outline();
        let mut doc = FastPdfDocument::from_bytes(&bytes).expect("open");
        assert_eq!(doc.page_count(), 1);
        let outline = doc.outline().unwrap();
        assert_eq!(outline.len(), 1);
        assert_eq!(outline[0].title, "Chapter 1");
        let rendered = doc.render_page(0, 1.0).unwrap();
        assert_eq!(rendered.width, 612);
        assert_eq!(rendered.pixels.len(), (612 * 792 * 4) as usize);
    }

    #[test]
    fn search_finds_text() {
        let bytes = minimal_pdf_with_outline();
        let mut doc = FastPdfDocument::from_bytes(&bytes).unwrap();
        let hits = doc.search("FastPDF").unwrap();
        assert!(!hits.is_empty());
    }
}
