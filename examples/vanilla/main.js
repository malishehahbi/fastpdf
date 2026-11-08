import { init, engineVersion, FastPdfDocument } from '@fastpdf/core';

await init();
document.getElementById('version').textContent = engineVersion();

/** @type {FastPdfDocument | null} */
let doc = null;
let page = 0;
const scale = 1.25;

const canvas = document.getElementById('canvas');
const sidebar = document.getElementById('sidebar');
const ctx = canvas.getContext('2d');

async function renderCurrent() {
  if (!doc) return;
  const image = await doc.renderPage(page, { scale });
  canvas.width = image.width;
  canvas.height = image.height;
  ctx.putImageData(image, 0, 0);
}

function renderOutline(items, depth = 0) {
  const ul = document.createElement('div');
  ul.className = 'outline';
  for (const item of items) {
    const btn = document.createElement('button');
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

document.getElementById('file').addEventListener('change', async (e) => {
  const file = e.target.files?.[0];
  if (!file) return;
  doc = await FastPdfDocument.openBytes(await file.arrayBuffer());
  page = 0;
  sidebar.replaceChildren();
  const outline = doc.getOutline();
  if (outline.length) {
    sidebar.appendChild(renderOutline(outline));
  }
  await renderCurrent();
  console.log('pages', doc.pageCount(), 'info', doc.getInfo());
});
