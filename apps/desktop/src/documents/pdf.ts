import { getDocument, GlobalWorkerOptions } from 'pdfjs-dist';
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url';

GlobalWorkerOptions.workerSrc = workerUrl;

export async function pdfText(file: File, signal?: AbortSignal) {
  let worker: import('tesseract.js').Worker | undefined;
  const recognized: number[] = [];
  const task = getDocument({ data: new Uint8Array(await file.arrayBuffer()), isEvalSupported: false });
  try {
    const doc = await task.promise;
    const pages: string[] = [];
    for (let i = 1; i <= doc.numPages; i++) {
      signal?.throwIfAborted();
      const page = await doc.getPage(i);
      const content = await page.getTextContent();
      let line = content.items.flatMap(item => 'str' in item ? [item.str, item.hasEOL ? '\n' : ''] : []).join('');
      if (!line.trim()) {
        signal?.throwIfAborted();
        const size = page.getViewport({ scale: 1 });
        const scale = Math.min(2.5, 3000 / Math.max(size.width, size.height));
        const viewport = page.getViewport({ scale });
        const canvas = document.createElement('canvas');
        canvas.width = Math.ceil(viewport.width);
        canvas.height = Math.ceil(viewport.height);
        try {
          await page.render({ canvas, viewport }).promise;
          const pixels = canvas.getContext('2d')!.getImageData(0, 0, canvas.width, canvas.height).data;
          if (pixels.every((v, index) => index % 4 === 3 || v === 255)) {
            pages.push('');
            page.cleanup();
            continue;
          }
          const { englishOcr } = await import('./ocr');
          worker ??= await englishOcr(signal);
          signal?.throwIfAborted();
          line = (await worker.recognize(canvas)).data.text.trim();
        } finally {
          canvas.width = canvas.height = 0;
        }
        if (!line) throw new Error(`Page ${i}: English OCR found no readable text. Check the page visually.`);
        recognized.push(i);
      }
      pages.push(line);
      page.cleanup();
    }
    return { text: pages.join('\n'), recognized };
  } finally {
    await Promise.all([task.destroy(), worker?.terminate()]);
  }
}
