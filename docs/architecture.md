# Architecture decision record

## PDF backend: **lopdf** (chosen)

| Criterion | lopdf | pdf-rs (`pdf` crate) |
|-----------|-------|----------------------|
| Mutate / save | Strong | Weaker for editing |
| Outlines & annots | Supported | Less ergonomic |
| Ecosystem fit | Used in fastpdf-core | Deferred |

**Spike result (synthetic fixtures):** both parsers open simple files; **lopdf** is the canonical model for outlines, annotation CRUD, AcroForm traversal, and incremental save.

## Rendering (browser / WASM)

Pipeline: **content streams → display list → tiny-skia** (see `render/interpreter.rs`).

- **Text:** lopdf `ToUnicode` / encodings for strings; **ab_glyph** outlines with PDF-embedded TTF (`FontFile2`) or a build-time default font (`build.rs`).
- **Images:** `/DeviceRGB`, `/DeviceGray`, `/DeviceCMYK`, `/Indexed`, JPEG/PNG via `image` crate; drawn in the PDF **1×1 unit square** × CTM (required for correct placement).
- **Forms:** nested XObject streams.

### Standard desktop alternative (not in WASM yet)

Industry-default rasterizers are **PDFium** or **MuPDF** (used by Chrome, many viewers). Pure Rust cannot match them on every PDF 1.7 file without years of operator work. For native-only high fidelity later, add optional `pdfium-render` behind a feature flag; **WASM builds keep the pure-Rust pipeline**.

## WASM boundary

`fastpdf-wasm` exposes `WasmDocument`; `packages/core` wraps it with `FastPdfDocument` and typed helpers.

## Page indices

JavaScript/Rust public API uses **0-based** page indices. lopdf `get_pages()` keys are normalized in `FastPdfDocument::from_document`.
