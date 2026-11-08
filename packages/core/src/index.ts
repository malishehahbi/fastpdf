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

  async renderPage(index: number, options: RenderOptions = {}): Promise<ImageData> {
    const scale = options.scale ?? 1.5;
    const result = this.doc.renderPage(index, scale);
    return renderResultToImageData(result);
  }

  search(query: string): unknown[] {
    return this.doc.search(query) as unknown[];
  }

  getAnnotations(index: number): unknown[] {
    return this.doc.getAnnotations(index) as unknown[];
  }

  getLinks(index: number): unknown[] {
    return this.doc.getLinks(index) as unknown[];
  }

  getTextSpans(index: number): unknown[] {
    return this.doc.getTextSpans(index) as unknown[];
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
