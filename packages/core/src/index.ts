import { version, WasmDocument, WasmRenderResult } from '../pkg/fastpdf.js';

let initPromise: Promise<void> | null = null;

/** Ensure the WASM module is loaded (bundler target auto-inits on first import). */
export async function init(): Promise<void> {
  if (!initPromise) {
    initPromise = import('../pkg/fastpdf.js').then(() => undefined);
  }
  await initPromise;
}

export function engineVersion(): string {
  return version();
}

export type OutlineItem = {
  title: string;
  pageIndex?: number;
  children: OutlineItem[];
};

export type PageInfo = {
  index: number;
  width: number;
  height: number;
  rotation: number;
  label?: string;
};

export type RenderOptions = { scale?: number };

export type TextSpan = {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  yTop?: number;
  fontSize?: number;
};

export type PageViewSize = { width: number; height: number };

export type SearchMatch = {
  pageIndex: number;
  start: number;
  end: number;
  snippet: string;
  rects?: { x: number; y: number; width: number; height: number }[];
};

export type LinkDto = {
  rect: { x: number; y: number; width: number; height: number };
  uri?: string;
  pageIndex?: number;
};

export class FastPdfDocument {
  private doc: WasmDocument;

  private constructor(doc: WasmDocument) {
    this.doc = doc;
  }

  static async openBytes(data: ArrayBuffer | Uint8Array): Promise<FastPdfDocument> {
    await init();
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    const doc = WasmDocument.openBytes(bytes);
    return new FastPdfDocument(doc);
  }

  static async openBytesWithPassword(
    data: ArrayBuffer | Uint8Array,
    password: string,
  ): Promise<FastPdfDocument> {
    await init();
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    const doc = WasmDocument.openBytesWithPassword(bytes, password);
    return new FastPdfDocument(doc);
  }

  static async openUrl(url: string, fetchOptions?: RequestInit): Promise<FastPdfDocument> {
    const res = await fetch(url, fetchOptions);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return FastPdfDocument.openBytes(await res.arrayBuffer());
  }

  pageCount(): number {
    return this.doc.pageCount();
  }

  getOutline(): OutlineItem[] {
    return this.doc.getOutline() as OutlineItem[];
  }

  getInfo(): Record<string, unknown> {
    return this.doc.getInfo() as Record<string, unknown>;
  }

  getPageInfo(index: number): PageInfo {
    return this.doc.getPageInfo(index) as PageInfo;
  }

  /** Hayro page size in points (aligned with the rendered bitmap). */
  getPageViewSize(index: number): PageViewSize {
    const wasmDoc = this.doc as WasmDocument & { getPageViewSize?: (i: number) => PageViewSize };
    if (typeof wasmDoc.getPageViewSize === 'function') {
      return wasmDoc.getPageViewSize(index) as PageViewSize;
    }
    const info = this.getPageInfo(index);
    return { width: info.width, height: info.height };
  }

  async renderPage(index: number, options: RenderOptions = {}): Promise<ImageData> {
    const scale = options.scale ?? 1.5;
    const result = this.doc.renderPage(index, scale);
    return renderResultToImageData(result);
  }

  /** Rasterize a page scaled so its longest side is at most `maxSide` points (hayro). */
  renderThumbnail(index: number, maxSide = 96): ImageData {
    const result = this.doc.renderThumbnail(index, maxSide);
    return renderResultToImageData(result);
  }

  getAnnotations(index: number): unknown[] {
    return this.doc.getAnnotations(index) as unknown[];
  }

  getLinks(index: number): LinkDto[] {
    return this.doc.getLinks(index) as LinkDto[];
  }

  getTextSpans(index: number): TextSpan[] {
    return this.doc.getTextSpans(index) as TextSpan[];
  }

  search(query: string): SearchMatch[] {
    return this.doc.search(query) as SearchMatch[];
  }

  getFormFields(): unknown[] {
    return this.doc.getFormFields() as unknown[];
  }

  setFormField(name: string, value: string): boolean {
    return this.doc.setFormField(name, value);
  }

  addHighlight(
    pageIndex: number,
    rect: { x: number; y: number; width: number; height: number },
    contents?: string,
  ): void {
    this.doc.addHighlight(pageIndex, rect.x, rect.y, rect.width, rect.height, contents ?? undefined);
  }

  saveToBytes(): Uint8Array {
    return this.doc.saveToBytes();
  }

  setMaxCacheEntries(n: number): void {
    this.doc.setMaxCacheEntries(n);
  }

  clearCache(): void {
    this.doc.clearCache();
  }

  renderTile(
    index: number,
    scale: number,
    tileX: number,
    tileY: number,
    tileW: number,
    tileH: number,
  ): ImageData {
    const result = this.doc.renderTile(index, scale, tileX, tileY, tileW, tileH);
    return renderResultToImageData(result);
  }
}

function renderResultToImageData(result: WasmRenderResult): ImageData {
  return new ImageData(
    new Uint8ClampedArray(result.pixels()),
    result.width,
    result.height,
  );
}

export { WasmDocument, WasmRenderResult };
