import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { WasmDocument } from '../../pkg/fastpdf.js';
import { engineVersion, FastPdfDocument, init } from '../index.js';

const here = dirname(fileURLToPath(import.meta.url));
const minimalPdf = readFileSync(join(here, '../../testdata/minimal.pdf'));

test('engineVersion is semver-like', () => {
  assert.match(engineVersion(), /^\d+\.\d+/);
});

test('WASM smoke: open, metadata, search, render', async () => {
  await init();
  const doc = await FastPdfDocument.openBytes(minimalPdf);

  assert.equal(doc.pageCount(), 1);

  const info = doc.getPageInfo(0);
  assert.ok(info.width > 0);
  assert.ok(info.height > 0);

  const hits = doc.searchPage(0, 'FastPDF');
  assert.ok(hits.length >= 1);
  assert.equal(hits[0].pageIndex, 0);

  const raw = WasmDocument.openBytes(minimalPdf);
  const rendered = raw.renderPage(0, 0.5);
  assert.ok(rendered.width > 0);
  assert.ok(rendered.height > 0);
  assert.ok(rendered.pixels().length >= rendered.width * rendered.height * 4);
});