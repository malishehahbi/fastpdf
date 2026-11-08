# FastPDF Roadmap

This document is the build plan for a **pure Rust** PDF engine targeting **WASM** and **framework-agnostic** JavaScript bindings. Timelines are indicative; adjust per contributor capacity.

**Legend:** ✅ done · 🚧 in progress · ⬜ not started

**Status (v0.1.0):** Phases 0–7 have **baseline implementations** in the repo (Rust core, WASM, `@fastpdf/core`, CLI, examples). Rendering, fonts, and full PDF 1.7 coverage remain incremental hardening—not every exit criterion in the tables below is production-grade yet. See [docs/architecture.md](./docs/architecture.md).

---

## Executive summary

| Phase | Focus | Outcome |
|-------|--------|---------|
| **0** | Scaffold | Workspace, CI, empty WASM loads in browser |
| **1** | Parse & model | Open PDF, pages, metadata, outlines |
| **2** | Render MVP | Page bitmap in WASM, canvas demo |
| **3** | Reader UX | Text layer, search, links, rotation |
| **4** | Annotations | Read + hit-test; then author/edit |
| **5** | Forms & security | AcroForm, passwords, permissions |
| **6** | Performance | Tiles, workers, streaming, memory caps |
| **7** | Ecosystem | React/Vue packages, docs site, 1.0 |

Pure Rust **rendering** is the longest pole; the roadmap front-loads parse/outline work and delivers a visible render path early, then deepens spec coverage.

---

## Rust crate strategy (pure Rust)

| Concern | Primary direction | Notes |
|---------|-------------------|--------|
| **PDF structure** | [`lopdf`](https://crates.io/crates/lopdf) and/or [`pdf`](https://crates.io/crates/pdf) (pdf-rs) | Evaluate both in Phase 0 spike; pick one **canonical** object model in `fastpdf-core`. |
| **Content streams** | Custom interpreter in `fastpdf-core` | PDF operators → display list (paths, text, images, transforms). |
| **Text extraction** | Built from display list + ToUnicode/CMap | Needed for selection and search. |
| **Rasterization** | [`tiny-skia`](https://crates.io/crates/tiny-skia) + image decode (`image` crate) | CPU raster, WASM-friendly; no GPU requirement. |
| **Fonts** | [`ab_glyph`](https://crates.io/crates/ab_glyph) / outline parsing | Type1, TrueType, composite fonts—incremental support. |
| **WASM boundary** | [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen) + `serde` for structured data | Keep heavy work in Rust; pass blobs/handles to JS. |
| **Encryption** | `lopdf` / `pdf` support + tests | Standard security handlers in Phase 5. |

**Non-goals for core:** binding PDFium, MuPDF, or other non-Rust renderers (optional separate feature crate later if community demands it—out of scope for “pure Rust” promise).

---

## Phase 0 — Foundation (1–2 weeks)

**Objective:** Monorepo that builds, tests, and loads WASM in a browser.

### Tasks

- ✅ Cargo workspace: `fastpdf-core`, `fastpdf-wasm`, `fastpdf-cli`
- ✅ `wasm-pack` pipeline: `packages/core/pkg` + TypeScript wrapper
- ✅ TypeScript: `init()`, `engineVersion()`, `FastPdfDocument`
- ✅ Examples: `examples/vanilla` (Vite), react/vue package stubs
- ✅ CI: `.github/workflows/ci.yml`
- ✅ Spike: **lopdf** — `docs/architecture.md`

### Exit criteria

- `npm run dev` in vanilla example prints WASM version in console
- `cargo test` passes on Windows/Linux/macOS

---

## Phase 1 — Document model & bookmarks (2–3 weeks)

**Objective:** Reliable open/read API without full render.

### Tasks

- ⬜ `FastPdfDocument::from_bytes` / incremental buffer API design
- ⬜ Page tree: count, `MediaBox`/`CropBox`, rotation
- ⬜ Document `Info` dictionary + XMP (basic)
- ⬜ **Outlines (bookmarks):** tree structure, titles, destinations (named + explicit)
- ⬜ Named destinations resolver
- ⬜ Error types mapped to JS (`InvalidPassword`, `CorruptPdf`, `UnsupportedFeature`)
- ⬜ Rust unit tests + golden JSON for outline trees

### JS API (draft)

```ts
interface OutlineItem {
  title: string;
  pageIndex?: number;
  children: OutlineItem[];
}
document.getPageCount(): number;
document.getOutline(): OutlineItem[];
```

### Exit criteria

- Demo: sidebar bookmark list navigates to `pageIndex` (placeholder canvas OK)

---

## Phase 2 — Rendering MVP (4–6 weeks)

**Objective:** Correct-enough rendering for common office PDFs at reasonable speed.

### Tasks

- ⬜ Content stream parser (common operators: `q/Q`, `cm`, `m/l/c/re`, `S/f/B`, `Do`, `Tj/TJ`, `Tf`, `rg/RG`, `gs`)
- ⬜ Display list intermediate representation
- ⬜ Rasterize to RGBA8 (`page.render({ scale })`)
- ⬜ Embedded images: DCT, Flate; stub exotic filters with clear errors
- ⬜ WASM: return `Uint8Array` or write to JS-owned buffer to reduce copies
- ⬜ Vanilla example: multi-page viewer, zoom, page prev/next

### Exit criteria

- Test corpus (10–20 PDFs): renders without panic; visual regression snapshots (Wasm or headless compare)
- Bookmarks from Phase 1 jump to rendered page

---

## Phase 3 — Reader essentials (3–4 weeks)

**Objective:** Behaviors users expect from a “PDF reader.”

### Tasks

- ⬜ Text positions → HTML text layer or JSON quads for selection
- ⬜ Search: case-insensitive, per-page index, highlight matches
- ⬜ Link annotations: hover cursor, internal + external URL
- ⬜ Page labels (`/PageLabels`)
- ⬜ Thumbnails API (small `scale` render cache)
- ⬜ Optional: `OffscreenCanvas` + worker wrapper design in TS (render in worker thread)

### Exit criteria

- Select/copy text on text-based PDFs
- Click in-doc links and outline entries

---

## Phase 4 — Annotations (4–6 weeks)

**Objective:** Full **read** path for standard annotation subtypes; **write** for common types.

### Read (first)

- ⬜ Enumerate annots per page: rect, contents, subtype, flags
- ⬜ Text, Link, Highlight, Underline, Squiggly, StrikeOut, Ink, Stamp, FreeText
- ⬜ Popup / IRT relationships
- ⬜ Hit-testing in page coordinates for UI tooltips

### Write (second)

- ⬜ Create highlight/underline from text quads
- ⬜ Ink strokes from pointer events
- ⬜ Save incremental update (`from_bytes` → modified bytes) via `lopdf`/core writer
- ⬜ Conflict policy: preserve foreign incremental updates

### Exit criteria

- Round-trip test: annotate → save → reopen → annots visible in FastPDF and a reference reader where possible

---

## Phase 5 — Forms & security (3–4 weeks)

**Objective:** AcroForm and encrypted documents.

### Tasks

- ⬜ Field tree: text, checkbox, radio, choice, button
- ⬜ Values, appearances (render widget annots)
- ⬜ Submit/export FDF/XFDF (subset)
- ⬜ Standard security handler: user password, owner password, permission bits
- ⬜ Redaction helper API (roadmap-friendly)

### Exit criteria

- Open password-protected test vectors
- Fill form fields and export updated PDF bytes

---

## Phase 6 — Performance & scale (ongoing)

**Objective:** Large files and smooth scrolling.

### Tasks

- ⬜ Tiled rendering (viewport + overscan)
- ⬜ LRU cache for pages and tiles (Rust-side + optional JS policy)
- ⬜ `SharedArrayBuffer` path where COOP/COEP allow
- ⬜ Linearized PDF / range request loader (fetch byte ranges)
- ⬜ Memory budget API (`setMaxCacheMb`)
- ⬜ Benchmarks: time-to-first-page, FPS while scrolling

### Exit criteria

- 500+ page synthetic doc: first page & scroll without OOM on typical laptop

---

## Phase 7 — Ecosystem & 1.0 (2–3 weeks)

**Objective:** Stable public API and framework ergonomics.

### Tasks

- ⬜ `@fastpdf/react`: `PdfViewer`, `useDocument`, `usePageBitmap`
- ⬜ `@fastpdf/vue` composable; Svelte/Angular examples in docs
- ⬜ API semver policy, changelog, migration guide
- ⬜ Documentation site: features matrix vs PDF 1.7 baseline
- ⬜ npm publish WASM + AGPL/MIT licensing decision finalized

### Exit criteria

- semver `1.0.0` with compatibility guarantees on TS types and WASM ABI version

---

## Testing strategy

| Layer | Approach |
|-------|----------|
| **Rust** | Unit tests per module; `insta` snapshots for display lists |
| **PDF fixtures** | Curated corpus in `tests/pdfs/` (small, redistributable); large files CI optional |
| **WASM** | `wasm-bindgen-test` for panic-free render on fixtures |
| **JS** | Vitest for loader and mock WASM |
| **Visual** | PNG diffs per page render (threshold for antialiasing) |

---

## Risk register

| Risk | Mitigation |
|------|------------|
| Pure Rust render slower than PDFium | Tile cache, worker, scale caps; document perf expectations |
| Font/CMap complexity | Tiered font support; fallbacks with visible tofu + logging |
| WASM binary size | `wasm-opt`, feature flags (`default-features = false`), split `parse` vs `render` features |
| Spec breadth | Publish **support matrix**; return `UnsupportedFeature` not silent wrong output |

---

## Suggested order of work (first 30 days)

```text
Week 1: Phase 0 complete + architecture spike (lopdf vs pdf)
Week 2: Phase 1 outlines + page metadata in JS API
Week 3–4: Phase 2 operator subset + first real page render
Week 5: Phase 3 text extraction MVP on simple PDFs
```

---

## Version tagging

- `0.1.0` — WASM loads, open PDF, outlines, metadata  
- `0.2.0` — Page render MVP  
- `0.3.0` — Text layer + search + links  
- `0.4.0` — Annotations read  
- `0.5.0` — Annotations write + save  
- `0.6.0` — Forms + encryption  
- `1.0.0` — API frozen, framework packages documented  

---

## How to use this roadmap

1. Complete **Phase 0** checklist in order.  
2. Keep `fastpdf-core` free of `wasm-bindgen` so all logic is testable with `cargo test`.  
3. Only expand JS API when Rust API is stable for that feature.  
4. Update this file at each phase exit: mark ✅ and link to PRs or tags.

For embedding examples, see [README.md](./README.md).
