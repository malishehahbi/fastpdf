# FastPDF

**FastPDF** is a **pure Rust** PDF engine compiled to **WebAssembly**, designed to run in the browser and in Node.js with the same API. It targets full reader capabilities—rendering, text selection, search, **annotations**, **bookmarks (outlines)**, forms, and document metadata—while staying **framework-agnostic** so you can embed it in React, Vue, Svelte, Angular, or vanilla JavaScript.

## Goals

| Principle | Description |
|-----------|-------------|
| **Pure Rust** | No native PDFium/MuPDF dependencies in the core; Rust crates for parse, mutate, and render pipeline. |
| **WASM-first** | One compiled core, thin JavaScript/TypeScript bindings via `wasm-bindgen`. |
| **Embeddable** | `@fastpdf/core` exposes a stable API; optional thin wrappers for React/Vue/etc. |
| **Reader-complete** | Outlines, links, annotations, AcroForm, permissions, incremental load, and print/export where spec allows. |

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│  Your app (React / Vue / Svelte / Angular / vanilla JS)      │
└──────────────────────────┬──────────────────────────────────┘
                           │ import @fastpdf/core
┌──────────────────────────▼──────────────────────────────────┐
│  TypeScript API (Document, Page, Annotation, Viewport)       │
│  + optional framework helpers (hooks / components)           │
└──────────────────────────┬──────────────────────────────────┘
                           │ wasm-bindgen
┌──────────────────────────▼──────────────────────────────────┐
│  fastpdf-core (Rust)                                         │
│  · parse & object model                                      │
│  · content stream interpreter → display list                 │
│  · rasterizer (CPU, WASM-safe)                               │
│  · annotations, outlines, forms, encryption                  │
└─────────────────────────────────────────────────────────────┘
```

## Repository layout (target)

```text
fastpdf/
├── crates/
│   ├── fastpdf-core/      # PDF logic (no wasm)
│   ├── fastpdf-wasm/      # wasm-bindgen exports
│   └── fastpdf-cli/       # dev tooling, benchmarks (optional)
├── packages/
│   ├── core/              # npm: WASM + TS types + loader
│   └── react/             # optional: <PdfViewer /> etc.
├── examples/
│   ├── vanilla/
│   ├── react/
│   └── vue/
├── ROADMAP.md
└── README.md
```

## Requirements

- **Rust** 1.75+ (you already have Rust installed)
- **wasm-pack** — `cargo install wasm-pack`
- **Node.js** 18+ (for the JS package and examples)
- **pnpm** or **npm** (monorepo scripts)

## Quick start

```bash
# From repo root
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Build WASM + TypeScript package
npm --prefix packages/core install
npm --prefix packages/core run build

# Run vanilla example
cd examples/vanilla
npm install
npm run dev
```

Page rendering uses **hayro** for fonts and rasterization; no `FASTPDF_FONT` or bundled TTF files are required to build `fastpdf-core`.

## Embedding in any framework

The **only required dependency** in your app is `@fastpdf/core`. Framework packages are optional sugar.

### Vanilla JavaScript

```javascript
import init, { FastPdfDocument } from '@fastpdf/core';

await init(); // loads WASM once

const doc = await FastPdfDocument.openUrl('/sample.pdf');
const page = doc.getPage(0);
const bitmap = await page.render({ scale: 1.5, format: 'rgba8' });

const canvas = document.getElementById('pdf-canvas');
const ctx = canvas.getContext('2d');
const imageData = new ImageData(new Uint8ClampedArray(bitmap), page.width, page.height);
ctx.putImageData(imageData, 0, 0);

console.log(doc.getOutline()); // bookmarks
console.log(page.getAnnotations());
```

### React

```tsx
import { useEffect, useRef } from 'react';
import init, { FastPdfDocument } from '@fastpdf/core';

export function PdfViewer({ url }: { url: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let doc: FastPdfDocument | null = null;
    (async () => {
      await init();
      doc = await FastPdfDocument.openUrl(url);
      const page = doc.getPage(0);
      const bitmap = await page.render({ scale: 2 });
      const canvas = canvasRef.current!;
      canvas.width = page.width;
      canvas.height = page.height;
      const ctx = canvas.getContext('2d')!;
      ctx.putImageData(
        new ImageData(new Uint8ClampedArray(bitmap), page.width, page.height),
        0, 0
      );
    })();
    return () => doc?.destroy();
  }, [url]);

  return <canvas ref={canvasRef} />;
}
```

### Vue 3

```vue
<script setup lang="ts">
import { ref, onMounted, watch } from 'vue';
import init, { FastPdfDocument } from '@fastpdf/core';

const props = defineProps<{ url: string }>();
const canvasRef = ref<HTMLCanvasElement | null>(null);

async function load() {
  await init();
  const doc = await FastPdfDocument.openUrl(props.url);
  const page = doc.getPage(0);
  const bitmap = await page.render({ scale: 2 });
  const canvas = canvasRef.value!;
  canvas.width = page.width;
  canvas.height = page.height;
  canvas.getContext('2d')!.putImageData(
    new ImageData(new Uint8ClampedArray(bitmap), page.width, page.height), 0, 0
  );
}

onMounted(load);
watch(() => props.url, load);
</script>

<template><canvas ref="canvasRef" /></template>
```

### Svelte

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import init, { FastPdfDocument } from '@fastpdf/core';

  export let url: string;
  let canvas: HTMLCanvasElement;

  onMount(async () => {
    await init();
    const doc = await FastPdfDocument.openUrl(url);
    const page = doc.getPage(0);
    const bitmap = await page.render({ scale: 2 });
    canvas.width = page.width;
    canvas.height = page.height;
    canvas.getContext('2d')!.putImageData(
      new ImageData(new Uint8ClampedArray(bitmap), page.width, page.height), 0, 0
    );
  });
</script>

<canvas bind:this={canvas}></canvas>
```

### Angular

```typescript
// pdf-viewer.component.ts
import { Component, ElementRef, Input, OnChanges, ViewChild } from '@angular/core';
import init, { FastPdfDocument } from '@fastpdf/core';

@Component({
  selector: 'app-pdf-viewer',
  template: '<canvas #c></canvas>',
})
export class PdfViewerComponent implements OnChanges {
  @Input() url = '';
  @ViewChild('c') canvasRef!: ElementRef<HTMLCanvasElement>;
  private ready = init();

  async ngOnChanges() {
    await this.ready;
    const doc = await FastPdfDocument.openUrl(this.url);
    const page = doc.getPage(0);
    const bitmap = await page.render({ scale: 2 });
    const canvas = this.canvasRef.nativeElement;
    canvas.width = page.width;
    canvas.height = page.height;
    canvas.getContext('2d')!.putImageData(
      new ImageData(new Uint8ClampedArray(bitmap), page.width, page.height), 0, 0
    );
  }
}
```

### SSR / bundler notes

- Use **dynamic `import()`** for `@fastpdf/core` on server-rendered routes; WASM only runs in the browser (or Node with wasm support).
- **Vite:** install `vite-plugin-wasm` and add it to `vite.config.js` (see `examples/vanilla`). The wasm-pack `bundler` output imports `.wasm` as ESM; Vite does not support that without this plugin.
- Next.js: load the viewer with `dynamic(() => import('./PdfViewer'), { ssr: false })`.

## Planned feature surface

| Area | Capabilities |
|------|----------------|
| **Navigation** | Page count, dimensions, rotation, labels |
| **Rendering** | RGBA tiles, scale, crop box, optional text layer for selection |
| **Outlines** | Bookmark tree, expand/collapse, go-to actions |
| **Links** | Internal destinations, URLs, rects |
| **Annotations** | Text, highlight, underline, ink, stamp, widgets (read + create/edit roadmap) |
| **Forms** | AcroForm field read; fill/submit in later phases |
| **Security** | User password, permission flags |
| **I/O** | `ArrayBuffer`, URL fetch, streaming/chunked load (roadmap) |

See **[ROADMAP.md](./ROADMAP.md)** for phases, milestones, and crate choices.

## Development

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cd packages/core && npm test
```
