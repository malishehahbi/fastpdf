import { init, engineVersion, FastPdfDocument } from '@fastpdf/core';

await init();
document.getElementById('version').textContent = engineVersion();

/** @type {FastPdfDocument | null} */
let doc = null;
let page = 0;
let scale = 1.25;
/** @type {{ width: number; height: number }} */
let pageView = { width: 612, height: 792 };
/** @type {{ x: number; y: number; width: number; height: number }[]} */
let activeHighlights = [];
/** @type {import('@fastpdf/core').SearchMatch[]} */
let lastSearchHits = [];

const canvas = document.getElementById('canvas');
const textLayer = document.getElementById('textLayer');
const highlightLayer = document.getElementById('highlightLayer');
const sidebar = document.getElementById('sidebar');
const thumbs = document.getElementById('thumbs');
const status = document.getElementById('status');
const errorEl = document.getElementById('error');
const toolbar = document.getElementById('toolbar');
const pageLabel = document.getElementById('pageLabel');
const zoom = document.getElementById('zoom');
const zoomLabel = document.getElementById('zoomLabel');
const searchInput = document.getElementById('search');
const searchResults = document.getElementById('searchResults');
const searchBtn = document.getElementById('searchBtn');
const measureCtx = document.createElement('canvas').getContext('2d');

const ARABIC = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/;

function showError(message) {
  errorEl.hidden = !message;
  errorEl.textContent = message ?? '';
}

function pdfRectToDom(rect) {
  const h = pageView.height;
  return {
    left: rect.x * scale,
    top: (h - rect.y - rect.height) * scale,
    width: rect.width * scale,
    height: rect.height * scale,
  };
}

function clientToPdf(clientX, clientY) {
  const box = canvas.getBoundingClientRect();
  const x = ((clientX - box.left) / box.width) * canvas.width / scale;
  const y = pageView.height - ((clientY - box.top) / box.height) * canvas.height / scale;
  return { x, y };
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

function paintHighlights() {
  highlightLayer.replaceChildren();
  for (const rect of activeHighlights) {
    const el = document.createElement('div');
    el.className = 'highlight';
    const dom = pdfRectToDom(rect);
    Object.assign(el.style, {
      left: `${dom.left}px`,
      top: `${dom.top}px`,
      width: `${dom.width}px`,
      height: `${dom.height}px`,
    });
    highlightLayer.appendChild(el);
  }
}

/**
 * @param {import('@fastpdf/core').TextSpan[]} spans
 */
function groupLines(spans) {
  const sorted = [...spans].sort((a, b) => {
    const dy = (a.yTop ?? 0) - (b.yTop ?? 0);
    if (Math.abs(dy) > 2) return dy;
    return a.x - b.x;
  });
  /** @type {{ yTop: number; rtl: boolean; spans: import('@fastpdf/core').TextSpan[] }[]} */
  const lines = [];
  for (const s of sorted) {
    const yTop = s.yTop ?? 0;
    const lh = s.fontSize || s.height || 12;
    const last = lines[lines.length - 1];
    if (!last || Math.abs(last.yTop - yTop) > lh * 0.45) {
      lines.push({ yTop, rtl: !!s.rtl, spans: [s] });
    } else {
      last.spans.push(s);
      if (s.rtl) last.rtl = true;
    }
  }
  for (const line of lines) {
    line.spans.sort((a, b) => (line.rtl ? b.x - a.x : a.x - b.x));
  }
  return lines;
}

function unionSpanBox(spans) {
  let x0 = Infinity;
  let yTop = Infinity;
  let x1 = -Infinity;
  let yBottom = -Infinity;
  let fontSize = 8;
  for (const s of spans) {
    const top = s.yTop > 0 ? s.yTop : pageView.height - s.y - s.height;
    const bottom = top + (s.fontSize || s.height);
    x0 = Math.min(x0, s.x);
    x1 = Math.max(x1, s.x + s.width);
    yTop = Math.min(yTop, top);
    yBottom = Math.max(yBottom, bottom);
    fontSize = Math.max(fontSize, s.fontSize || s.height);
  }
  return { x: x0, yTop, width: x1 - x0, height: yBottom - yTop, fontSize };
}

function buildTextLayer(spans) {
  textLayer.replaceChildren();
  textLayer.style.width = `${canvas.width}px`;
  textLayer.style.height = `${canvas.height}px`;
  for (const line of groupLines(spans)) {
    let text = '';
    for (const s of line.spans) {
      if (s.spaceBefore) text += ' ';
      text += s.text;
    }
  if (!text.trim()) continue;

    const box = unionSpanBox(line.spans);
    const el = document.createElement('span');
    el.textContent = text;
    const left = box.x * scale;
    const top = box.yTop * scale;
    const width = Math.max(1, box.width * scale);
    const height = Math.max(1, box.fontSize * scale);
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
    el.style.width = `${width}px`;
    el.style.height = `${height}px`;
    el.style.fontSize = `${height}px`;
    el.style.lineHeight = `${height}px`;
    el.style.direction = line.rtl ? 'rtl' : 'ltr';
    el.style.unicodeBidi = 'plaintext';
    const complex = line.rtl || ARABIC.test(text);
    if (!complex && measureCtx && text.length > 1) {
      measureCtx.font = `${height}px sans-serif`;
      const natural = measureCtx.measureText(text).width;
      if (natural > 1 && width / natural > 0.55 && width / natural < 3.5) {
        el.style.transform = `scaleX(${width / natural})`;
        el.style.transformOrigin = line.rtl ? '100% 0%' : '0% 0%';
      }
    }
    textLayer.appendChild(el);
  }
}

async function renderCurrent() {
  if (!doc) return;
  showError(null);
  try {
    const image = await doc.renderPage(page, { scale });
    pageView = doc.getPageViewSize(page);
    canvas.width = image.width;
    canvas.height = image.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('2D context unavailable');
    ctx.putImageData(image, 0, 0);

    const spans = doc.getTextSpans(page);
    buildTextLayer(spans);
    if (lastSearchHits.length) {
      applyHighlightsForCurrentPage();
    } else {
      paintHighlights();
    }

    const links = doc.getLinks(page);
    canvas.style.cursor = 'default';
    canvas.dataset.links = JSON.stringify(links);

    status.textContent = `Rendered ${image.width}×${image.height} px · ${groupLines(spans).length} text lines`;
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
      activeHighlights = [];
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
        activeHighlights = [];
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

function applyHighlightsForCurrentPage() {
  activeHighlights = lastSearchHits.filter((h) => h.pageIndex === page).flatMap((h) => h.rects ?? []);
  paintHighlights();
}

function renderSearchResults(hits) {
  searchResults.replaceChildren();
  for (const hit of hits.slice(0, 40)) {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'search-hit';
    btn.textContent = `p.${hit.pageIndex + 1}: ${hit.snippet}`;
    btn.addEventListener('click', () => {
      activeHighlights = hit.rects ?? [];
      page = hit.pageIndex;
      void renderCurrent();
    });
    searchResults.appendChild(btn);
  }
}

async function runSearch() {
  if (!doc) return;
  const q = searchInput.value.trim();
  lastSearchHits = [];
  activeHighlights = [];
  searchResults.replaceChildren();
  if (!q) {
    paintHighlights();
    return;
  }
  searchBtn.disabled = true;
  searchBtn.textContent = '…';
  await new Promise((r) => requestAnimationFrame(r));
  try {
    lastSearchHits = doc.search(q);
    renderSearchResults(lastSearchHits);
    if (lastSearchHits.length === 0) {
      status.textContent = `No matches for “${q}”`;
    } else {
      status.textContent = `${lastSearchHits.length} match(es) for “${q}”`;
      const onPage = lastSearchHits.some((h) => h.pageIndex === page);
      if (!onPage) {
        page = lastSearchHits[0].pageIndex;
        await renderCurrent();
      } else {
        applyHighlightsForCurrentPage();
      }
    }
  } finally {
    searchBtn.disabled = false;
    searchBtn.textContent = 'Find';
  }
}

document.getElementById('prev').addEventListener('click', () => {
  if (!doc || page <= 0) return;
  page -= 1;
  activeHighlights = [];
  renderCurrent();
});

document.getElementById('next').addEventListener('click', () => {
  if (!doc || page >= doc.pageCount() - 1) return;
  page += 1;
  activeHighlights = [];
  renderCurrent();
});

zoom.addEventListener('input', () => {
  scale = Number(zoom.value);
  zoomLabel.textContent = `${scale.toFixed(2)}×`;
  renderCurrent();
});

searchInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') void runSearch();
});
searchBtn.addEventListener('click', () => void runSearch());

canvas.addEventListener('mousemove', (e) => {
  if (!doc) return;
  const { x, y } = clientToPdf(e.clientX, e.clientY);
  /** @type {import('@fastpdf/core').LinkDto[]} */
  const links = JSON.parse(canvas.dataset.links || '[]');
  const over = links.some(
    (l) =>
      x >= l.rect.x &&
      x <= l.rect.x + l.rect.width &&
      y >= l.rect.y &&
      y <= l.rect.y + l.rect.height,
  );
  canvas.style.cursor = over ? 'pointer' : 'default';
});

canvas.addEventListener('click', (e) => {
  if (!doc) return;
  const { x, y } = clientToPdf(e.clientX, e.clientY);
  for (const link of doc.getLinks(page)) {
    const r = link.rect;
    if (x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height) {
      if (link.uri) window.open(link.uri, '_blank', 'noopener,noreferrer');
      else if (link.pageIndex != null) {
        page = link.pageIndex;
        activeHighlights = [];
        void renderCurrent();
      }
      return;
    }
  }
});

document.getElementById('file').addEventListener('change', async (e) => {
  const file = e.target.files?.[0];
  if (!file) return;
  showError(null);
  try {
    doc = await FastPdfDocument.openBytes(await file.arrayBuffer());
    page = 0;
    activeHighlights = [];
    lastSearchHits = [];
    sidebar.replaceChildren();
    thumbs.replaceChildren();
    searchResults.replaceChildren();
    const outline = doc.getOutline();
    if (outline.length) {
      sidebar.appendChild(renderOutline(outline));
    } else {
      sidebar.textContent = 'No bookmarks';
    }
    await buildThumbnails();
    await renderCurrent();
  } catch (err) {
    console.error(err);
    showError(err instanceof Error ? err.message : String(err));
  }
});
