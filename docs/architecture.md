# Architecture decision record

## PDF backend: **lopdf** (chosen)

| Criterion | lopdf | pdf-rs (`pdf` crate) |
|-----------|-------|----------------------|
| Mutate / save | Strong | Weaker for editing |
| Outlines & annots | Supported | Less ergonomic |
| Ecosystem fit | Used in fastpdf-core | Deferred |

**Spike result (synthetic fixtures):** both parsers open simple files; **lopdf** is the canonical model for outlines, annotation CRUD, AcroForm traversal, and incremental save.

## Rendering

Page bitmaps are produced by **[hayro](https://docs.rs/hayro) 0.7** with the `embed-fonts` feature. That is the same pure-Rust rasterizer [PdfCraft](https://github.com/storytold/pdfcraft) uses for viewing (their README: “pages are drawn by the hayro crate”). It handles real fonts, images (JPEG, JPEG 2000, JBIG2, CCITT), and most content streams.

`lopdf` remains the model for outlines, annotations, forms, and saving. Hayro reads the original PDF bytes kept on the document.

`FastPdfDocument` keeps a lazily opened **`HayroSession`** (parsed `Pdf`) for the lifetime of the document. After `save_to_bytes` or structural edits, the session is dropped so the next render uses updated bytes. Password-protected files pass the user password into hayro while lopdf handles metadata decrypt.

**Text:** `render::text_extract` runs hayro’s `interpret_page` with a glyph `Device` (PdfCraft-style, simplified) for positioned `TextSpan`s. Plain-text search merges hayro and lopdf streams when they differ; highlight rects come from hayro spans.

## WASM boundary

`fastpdf-wasm` exposes `WasmDocument`; `packages/core` wraps it with `FastPdfDocument` and typed helpers.

## Page indices

JavaScript/Rust public API uses **0-based** page indices. lopdf `get_pages()` keys are normalized in `FastPdfDocument::from_document`.
