import { init, engineVersion, FastPdfDocument } from '@fastpdf/core';

await init();
document.getElementById('version').textContent = engineVersion();

/** @type {FastPdfDocument | null} */
let doc = null;
let page = 0;
let scale = 1.25;

const canvas = document.getElementById('canvas');
const sidebar = document.getElementById('sidebar');
const thumbs = document.getElementById('thumbs');
const status = document.getElementById('status');
const errorEl = document.getElementById('error');
const pager = document.getElementById('pager');
const toolbar = document.getElementById('toolbar');
const pageLabel = document.getElementById('pageLabel');
const zoom = document.getElementById('zoom');
const zoomLabel = document.getElementById('zoomLabel');

function showError(message) {
  errorEl.hidden = !message;
  errorEl.textContent = message ?? '';
}

function updatePager() {
  if (!doc) {
    toolbar.hidden = true;
    return;
  }
  toolbar.hidden = false;
  pageLabel.textContent = `Page ${page + 1} / ${doc.pageCount()}`;
  zoomLabel.textContent = `${scale.toFixed(2)}×`;
  for (const btn of thumbs.querySelectorAll('button')) {
    btn.classList.toggle('active', Number(btn.dataset.page) === page);
  }
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
    status.textContent = `Rendered ${image.width}×${image.height} px · page ${info.width.toFixed(0)}×${info.height.toFixed(0)} pt (hayro)`;
    updatePager();
  } catch (err) {
    console.error(err);
    showError(err instanceof Error ? err.message : String(err));
    status.textContent = 'Render failed.';
  }
}

async function buildThumbnails() {
  if (!doc) return;
  thumbs.replaceChildren();
  const count = doc.pageCount();
  for (let i = 0; i < count; i += 1) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.dataset.page = String(i);
    btn.title = `Page ${i + 1}`;
    const mini = document.createElement('canvas');
    btn.appendChild(mini);
    btn.addEventListener('click', () => {
      page = i;
      renderCurrent();
    });
    thumbs.appendChild(btn);
    try {
      const image = doc.renderThumbnail(i, 96);
      mini.width = image.width;
      mini.height = image.height;
      mini.getContext('2d')?.putImageData(image, 0, 0);
    } catch {
      btn.textContent = `${i + 1}`;
    }
  }
  updatePager();
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

zoom.addEventListener('input', () => {
  scale = Number(zoom.value);
  zoomLabel.textContent = `${scale.toFixed(2)}×`;
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
    thumbs.replaceChildren();
    const outline = doc.getOutline();
    if (outline.length) {
      sidebar.appendChild(renderOutline(outline));
    } else {
      sidebar.textContent = 'No bookmarks';
    }
    await buildThumbnails();
    await renderCurrent();
    console.log('pages', doc.pageCount(), 'info', doc.getInfo());
  } catch (err) {
    console.error(err);
    showError(err instanceof Error ? err.message : String(err));
  }
});
