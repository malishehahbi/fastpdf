import { init, engineVersion, FastPdfDocument } from '@fastpdf/core';

await init();
document.getElementById('version').textContent = engineVersion();

/** @type {FastPdfDocument | null} */
let doc = null;
let page = 0;
const scale = 1.25;

const canvas = document.getElementById('canvas');
const sidebar = document.getElementById('sidebar');
const status = document.getElementById('status');
const errorEl = document.getElementById('error');
const pager = document.getElementById('pager');
const pageLabel = document.getElementById('pageLabel');

function showError(message) {
  errorEl.hidden = !message;
  errorEl.textContent = message ?? '';
}

function updatePager() {
  if (!doc) {
    pager.hidden = true;
    return;
  }
  pager.hidden = false;
  pageLabel.textContent = `Page ${page + 1} / ${doc.pageCount()}`;
}

async function renderCurrent() {
  if (!doc) return;
  showError(null);
  try {
    const image = await doc.renderPage(page, { scale });
    const info = doc.getPageInfo(page);
    canvas.width = image.width;
    canvas.height = image.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D context unavailable');
    ctx.putImageData(image, 0, 0);
    const spans = doc.getTextSpans(page);
    status.textContent = `Rendered ${image.width}×${image.height} px · page ${info.width.toFixed(0)}×${info.height.toFixed(0)} pt · ${spans.length} text span(s) — complex Office PDFs may still look incomplete until font support improves.`;
    updatePager();
  } catch (err) {
    console.error(err);
    showError(err instanceof Error ? err.message : String(err));
    status.textContent = 'Render failed.';
  }
}

function renderOutline(items, depth = 0) {
  const ul = document.createElement('div');
  ul.className = 'outline';
  for (const item of items) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.textContent = `${'  '.repeat(depth)}${item.title}`;
    btn.addEventListener('click', () => {
      if (item.pageIndex != null) {
        page = item.pageIndex;
        renderCurrent();
      }
    });
    ul.appendChild(btn);
    if (item.children?.length) {
      ul.appendChild(renderOutline(item.children, depth + 1));
    }
  }
  return ul;
}

document.getElementById('prev').addEventListener('click', () => {
  if (!doc || page <= 0) return;
  page -= 1;
  renderCurrent();
});

document.getElementById('next').addEventListener('click', () => {
  if (!doc || page >= doc.pageCount() - 1) return;
  page += 1;
  renderCurrent();
});

document.getElementById('file').addEventListener('change', async (e) => {
  const file = e.target.files?.[0];
  if (!file) return;
  showError(null);
  try {
    doc = await FastPdfDocument.openBytes(await file.arrayBuffer());
    page = 0;
    sidebar.replaceChildren();
    const outline = doc.getOutline();
    if (outline.length) {
      sidebar.appendChild(renderOutline(outline));
    } else {
      sidebar.textContent = 'No bookmarks';
    }
    await renderCurrent();
    console.log('pages', doc.pageCount(), 'info', doc.getInfo());
  } catch (err) {
    console.error(err);
    showError(err instanceof Error ? err.message : String(err));
  }
});
