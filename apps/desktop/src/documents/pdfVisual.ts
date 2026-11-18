import { getDocument, type PDFDocumentProxy } from 'pdfjs-dist';
import './pdf'; // Configure the same bundled PDF.js worker as text review.

async function raster(doc: PDFDocumentProxy, pageNumber: number, scale: number) {
  if (pageNumber > doc.numPages) return null;
  const page = await doc.getPage(pageNumber);
  const viewport = page.getViewport({ scale });
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.ceil(viewport.width));
  canvas.height = Math.max(1, Math.ceil(viewport.height));
  try {
    await page.render({ canvas, viewport, background: 'white' }).promise;
    return canvas;
  } finally { page.cleanup(); }
}

export interface VisualPage { number: number; status: string; preview?: Blob }

export async function comparePdfPages(left: File, right: File, signal: AbortSignal, append: (p: VisualPage) => void) {
  const tasks = (await Promise.all([left.arrayBuffer(), right.arrayBuffer()])).map(data => getDocument({ data, isEvalSupported: false }));
  try {
    const [a, b] = await Promise.all(tasks.map(t => t.promise));
    for (let n = 1; n <= Math.max(a.numPages, b.numPages); n++) {
      signal.throwIfAborted();
      const sizes = await Promise.all([a, b].map(async d => n <= d.numPages ? (await d.getPage(n)).getViewport({ scale: 1 }) : null));
      const longest = Math.max(...sizes.map(s => s ? Math.max(s.width, s.height) : 1));
      const scale = Math.min(2, 1400 / longest);
      const [old, revised] = await Promise.all([raster(a, n, scale), raster(b, n, scale)]);
      signal.throwIfAborted();
      if (!old || !revised) {
        const canvas = (old ?? revised)!;
        const preview = await new Promise<Blob | null>(resolve => canvas.toBlob(resolve));
        append({ number: n, status: old ? 'Removed' : 'Added', preview: preview ?? undefined });
        continue;
      }
      const width = Math.max(old.width, revised.width), height = Math.max(old.height, revised.height);
      const before = old.getContext('2d')!.getImageData(0, 0, width, height).data;
      const result = revised.getContext('2d')!.getImageData(0, 0, width, height);
      const after = result.data;
      let changed = sizes[0]!.width !== sizes[1]!.width || sizes[0]!.height !== sizes[1]!.height;
      for (let i = 0; i < after.length; i += 4) {
        const different = before[i] !== after[i] || before[i + 1] !== after[i + 1] || before[i + 2] !== after[i + 2] || before[i + 3] !== after[i + 3];
        if (different) {
          changed = true;
          after[i] = 209; after[i + 1] = 28; after[i + 2] = 101; after[i + 3] = 255;
        }
      }
      if (changed) {
        revised.width = width; revised.height = height;
        revised.getContext('2d')!.putImageData(result, 0, 0);
        const preview = await new Promise<Blob | null>(resolve => revised.toBlob(resolve));
        append({ number: n, status: 'Changed', preview: preview ?? undefined });
      } else append({ number: n, status: 'Unchanged' });
      old.width = revised.width = 0;
    }
  } finally { await Promise.all(tasks.map(t => t.destroy())); }
}
