import type { VisualPage } from './pdfVisual';
import type { ReviewReport } from './reviewReport';

function embedPreview(blob: Blob, number: number) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    const failed = () => reject(new Error(`Could not embed the preview for page ${number}.`));
    reader.onerror = failed;
    reader.onload = () => {
      if (typeof reader.result === 'string' && reader.result.startsWith('data:image/png;base64,')) resolve(reader.result);
      else failed();
    };
    reader.readAsDataURL(blob);
  });
}

const descriptions: Record<string, string> = {
  Changed: 'The rendered pixels or page dimensions differ at this position. The overlay shows the revised page with differing pixels in pink.',
  Added: 'Only the revision has a page at this position. The image shows the revised page.',
  Removed: 'Only the original has a page at this position. The image shows the original page.',
};

export async function visualReport(pages: VisualPage[], signal: AbortSignal): Promise<ReviewReport> {
  const counts: Record<string, number> = { Changed: 0, Added: 0, Removed: 0, Unchanged: 0 };
  const items: ReviewReport['items'] = [];
  for (const page of pages) {
    signal.throwIfAborted();
    if (!Object.hasOwn(counts, page.status)) throw new Error(`Unknown comparison result for page ${page.number}.`);
    counts[page.status]++;
    if (page.status === 'Unchanged') continue;
    if (!page.preview) throw new Error(`The preview for page ${page.number} is unavailable. Reopen the comparison and try again.`);
    items.push({
      title: `Page ${page.number} · ${page.status}`,
      description: descriptions[page.status],
      image: await embedPreview(page.preview, page.number),
    });
  }
  signal.throwIfAborted();
  return {
    mode: 'PDF visual comparison',
    summary: counts.Unchanged === pages.length
      ? `All page positions matched at preview resolution. ${counts.Unchanged} ${pages.length === 1 ? 'page' : 'pages'} unchanged.`
      : `${pages.length} page ${pages.length === 1 ? 'position' : 'positions'} compared. ${counts.Changed} changed, ${counts.Added} added, ${counts.Removed} removed and ${counts.Unchanged} unchanged.`,
    notes: [
      'Pages are paired by position. An insertion can shift the comparison of all later pages.',
      'Each pair is rendered at the same scale with a maximum long edge of 1,400 pixels. Finer differences can fall below this resolution.',
      'This report compares page appearance. Use text review for wording changes and English OCR results.',
    ],
    items,
  };
}
