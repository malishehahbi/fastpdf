import { useEffect, useRef, useState } from 'react';
import { FastPdfDocument } from '@fastpdf/core';

export type PdfViewerProps = {
  url: string;
  page?: number;
  scale?: number;
  className?: string;
};

export function PdfViewer({ url, page = 0, scale = 1.5, className }: PdfViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let doc: FastPdfDocument | null = null;
    (async () => {
      try {
        doc = await FastPdfDocument.openUrl(url);
        const image = await doc.renderPage(page, { scale });
        const canvas = canvasRef.current;
        if (!canvas) return;
        canvas.width = image.width;
        canvas.height = image.height;
        canvas.getContext('2d')?.putImageData(image, 0, 0);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    })();
    return () => {
      doc?.clearCache();
    };
  }, [url, page, scale]);

  if (error) {
    return <div role="alert">Failed to load PDF: {error}</div>;
  }

  return <canvas ref={canvasRef} className={className} aria-label="PDF page" />;
}
