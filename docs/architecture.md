# Architecture decision record

## PDF backend: **lopdf** (chosen)

| Criterion | lopdf | pdf-rs (`pdf` crate) |
|-----------|-------|----------------------|
| Mutate / save | Strong | Weaker for editing |
| Outlines & annots | Supported | Less ergonomic |
| Ecosystem fit | Used in fastpdf-core | Deferred |

**Spike result (synthetic fixtures):** both parsers open simple files; **lopdf** is the canonical model for outlines, annotation CRUD, AcroForm traversal, and incremental save.

## Rendering

Content streams are interpreted in `fastpdf-core/src/render/interpreter.rs` into a display list, rasterized with **tiny-skia** (CPU, WASM-safe).

Limitations at 0.1.0:

- Subset of PDF operators (common path/text/image ops).
- Text draw uses placeholder quads until full font embedding lands.
- Exotic image filters return errors at decode time.

## WASM boundary

`fastpdf-wasm` exposes `WasmDocument`; `packages/core` wraps it with `FastPdfDocument` and typed helpers.

## Page indices

JavaScript/Rust public API uses **0-based** page indices. lopdf `get_pages()` keys are normalized in `FastPdfDocument::from_document`.
