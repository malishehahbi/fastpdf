# FastPDF Roadmap

FastPDF is a **pure Rust** PDF engine for **WASM** and **embeddable JS** (React, Vue, Svelte, Angular, vanilla). The plan follows the same *layered* idea as [PdfCraft](https://github.com/storytold/pdfcraft): separate **render**, **edit/model**, and **bindings**, and use a production rasterizer (hayro) instead of reinventing PDF painting.

**Legend:** ✅ done · 🚧 in progress · ⬜ not started

**Current stage:** alpha **0.3.0-alpha** — hayro viewing + hayro text layer (M3 started); edit/save via lopdf.

---

## Architecture (PdfCraft-inspired)

PdfCraft splits concerns so the core never depends on a UI. FastPDF targets a smaller subset, same discipline:

| Layer | FastPDF today | Target crate (future) | Responsibility |
|-------|---------------|----------------------|----------------|
| **Raster** | ✅ `hayro` in `fastpdf-core::render` | `fastpdf-render` | Page pixels, tiles, thumbnails, (later) hayro text device |
| **Model / edit** | ✅ `lopdf` in `fastpdf-core` | `fastpdf-model` | Open, outlines, annots, AcroForm, incremental save |
| **Wasm / JS** | ✅ `fastpdf-wasm`, `@fastpdf/core` | `packages/core` | Stable TS API, framework-agnostic |
| **Engine façade** | 🚧 `FastPdfDocument` | `fastpdf-engine` | Sessions, cache, passwords, one API for all frontends |
| **Filters / crypt** | ⬜ via lopdf/hayro | optional split | Stream filters, security handler (when we outgrow defaults) |
| **UI** | ✅ `examples/vanilla`, stubs | `packages/react` etc. | Demos only; apps bring their own UI |

**Rendering (important):** PdfCraft’s README states pages are **drawn by hayro** while a native painter is built. FastPDF adopts the same strategy: **hayro 0.7 + `embed-fonts`** for all page bitmaps. **lopdf** is not used for pixels.

See [docs/architecture.md](./docs/architecture.md).

---

## Milestone map

| Milestone | Theme | Outcome |
|-----------|--------|---------|
| **M0** | Scaffold | Workspace, WASM, Vite example |
| **M1** | Model | lopdf: pages, outlines, metadata, errors |
| **M2** | View | hayro: multi-page viewer, zoom, cache |
| **M3** | Reader | Text layer, search, links, labels |
| **M4** | Annotations | Read all common types; write highlight/ink |
| **M5** | Forms & crypto | AcroForm fill, hayro+lopdf passwords |
| **M6** | Performance | Tiles, workers, range load, memory caps |
| **M7** | 1.0 & ecosystem | semver API, framework packages, docs site |

---

## M0 — Foundation ✅

- ✅ `fastpdf-core`, `fastpdf-wasm`, `fastpdf-cli`
- ✅ `@fastpdf/core`, `wasm-pack`, Vite + `vite-plugin-wasm`
- ✅ CI (Rust + wasm build)
- ✅ ADR: lopdf for edit, hayro for render

---

## M1 — Document model ✅ (lopdf)

- ✅ `from_bytes`, `from_bytes_with_password` (lopdf decrypt)
- ✅ Page count, MediaBox, rotation, page labels
- ✅ Outlines (bookmark tree + destinations)
- ✅ Info dictionary (UTF-16 aware)
- ✅ Permissions / encrypted flag
- ✅ WASM error mapping
- ⬜ Named destinations (full tree)
- ⬜ XMP metadata block

**Exit:** Open corpus PDFs; outline sidebar navigates by `pageIndex`.

---

## M2 — Viewing ✅

- ✅ hayro page render (RGBA for canvas)
- ✅ Vanilla example: file picker, prev/next, outlines
- ✅ **Hayro session:** reuse `Pdf` per document (invalidated on save)
- ✅ Encrypted files: hayro `new_with_password` with stored password
- ✅ Tile crop (viewport regions) for large pages
- ✅ Thumbnails API (`maxSide` scale)
- ✅ Zoom + thumbnail strip in vanilla demo
- ✅ MediaBox inheritance + PDF text-string decoding (bookmarks)
- ⬜ CLI `fastpdf render` uses hayro (not legacy path)

**Exit:** Office / Word / ONLYOFFICE PDFs match desktop viewer for text and images.

---

## M3 — Reader UX

Inspired by PdfCraft `pdfcraft-render` text device (hayro `Device`, reading order).

- ✅ Hayro text extraction (`render::text_extract`, glyph boxes → `TextSpan`)
- ✅ HTML text layer in vanilla (select/copy)
- ✅ Search with hayro highlight rects (lopdf fallback for plain search text)
- ✅ Link hit-test + cursor (internal + URI) in vanilla
- 🚧 Reading order, line grouping, full PdfCraft `PageText` parity
- ⬜ Optional: `OffscreenCanvas` worker wrapper in `@fastpdf/core`

**Exit:** Select/copy text on hayro-extracted spans; click links.

---

## M4 — Annotations

- ✅ Read list + link subset via lopdf
- ✅ Hit-test rects
- 🚧 Highlight create + `save_to_bytes`
- ⬜ Ink, FreeText, Stamp appearances (lopdf + appearances)
- ⬜ Popup / IRT / reply threads
- ⬜ Round-trip tests vs reference reader

---

## M5 — Forms & security

- ✅ Basic AcroForm enumeration + set field value
- ⬜ Widget appearances in hayro render (form XObject)
- ⬜ XFDF export/import (subset)
- ⬜ Owner vs user password; permission bits enforced in API
- ⬜ Linearized / encrypted save policy

---

## M6 — Performance & scale

PdfCraft: tile cache, render stats, corpus gates.

- 🚧 LRU page bitmap cache (Rust)
- ⬜ Hayro tile API (crop or upstream offsets when vendored)
- ⬜ `SharedArrayBuffer` render worker (TS)
- ⬜ HTTP range / linearized load
- ⬜ `setMaxCacheEntries` / memory budget docs
- ⬜ Bench: time-to-first-page, 500-page scroll

---

## M7 — Ecosystem & 1.0

- 🚧 `@fastpdf/react` `PdfViewer`
- 🚧 `@fastpdf/vue` `usePdfDocument`
- ⬜ Svelte / Angular full examples
- ⬜ Feature matrix vs PDF 1.7
- ⬜ npm publish, changelog, semver policy for WASM ABI

---

## Testing strategy (PdfCraft-style)

| Layer | Approach |
|-------|----------|
| **Unit** | lopdf outline/metadata; hayro render smoke on tiny PDF |
| **Corpus** | Redistributable fixtures in `tests/pdfs/` (+ optional pdf.js subset later) |
| **WASM** | `wasm-pack test` / headless render in CI |
| **Visual** | CLI PNG output diff (threshold) |
| **Oracle** | Later: compare text with poppler/hayro on CI optional job |

---

## Risks

| Risk | Mitigation |
|------|------------|
| WASM binary size (hayro) | `wasm-opt`, lazy init, document cache limits |
| Dual parser (lopdf + hayro) | Clear split: edit vs render; invalidate hayro session on save |
| Password / encrypt edge cases | hayro open + lopdf decrypt; test vectors |
| Text selection before M3 | Document “search is approximate until hayro text lands” |

---

## Version tags

- **0.1.0** — Scaffold + lopdf model + toy renderer (superseded)
- **0.2.0** — hayro viewing ✅
- **0.2.x** — 🚧 M2 session, tiles, thumbnails, viewer polish
- **0.3.0** — M3 reader (hayro text layer)
- **0.4.0** — M4 annotations write
- **0.5.0** — M5 forms/crypto
- **1.0.0** — M7 API freeze

---

## Next implementation slice (active)

1. Port PdfCraft line layout + case-folded search on glyph indices.
2. CLI `fastpdf render` / `fastpdf text` via hayro.
3. M4: highlight write + round-trip tests.
4. M5: form appearances visible in hayro render.
5. M6: render worker + tile cache tuning.

Update this file when a milestone exits.
