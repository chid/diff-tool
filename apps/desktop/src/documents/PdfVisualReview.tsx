import { useLayoutEffect, useState } from 'react';
import { comparePdfPages, type VisualPage } from './pdfVisual';
import { downloadReviewReport } from './reviewReport';
import { visualReport } from './visualReport';

export default function PdfVisualReview({ original, revised }: { original: File; revised: File }) {
  const [pages, setPages] = useState<(VisualPage & { url?: string })[]>([]);
  const [status, setStatus] = useState('Comparing pages…');
  const [completed, setCompleted] = useState<AbortSignal>();
  const [saving, setSaving] = useState(false);
  const [exportError, setExportError] = useState('');
  // Invalidate an in-flight export as soon as a different file pair is committed.
  useLayoutEffect(() => {
    const controller = new AbortController();
    const urls: string[] = [];
    setPages([]);
    setStatus('Comparing pages…');
    setCompleted(undefined);
    setSaving(false);
    setExportError('');
    comparePdfPages(original, revised, controller.signal, p => {
      if (controller.signal.aborted) return;
      const url = p.preview ? URL.createObjectURL(p.preview) : undefined;
      if (url) urls.push(url);
      setPages(prev => [...prev, { ...p, url }]);
    }).then(() => {
      if (controller.signal.aborted) return;
      setStatus('Page comparison complete');
      setCompleted(controller.signal);
    })
      .catch((e: unknown) => { if (!controller.signal.aborted) setStatus(e instanceof Error ? e.message : String(e)); });
    return () => { controller.abort(); urls.forEach(url => URL.revokeObjectURL(url)); };
  }, [original, revised]);
  async function saveReport() {
    if (!completed || completed.aborted || saving) return;
    setSaving(true);
    setExportError('');
    try {
      const report = await visualReport(pages, completed);
      if (!completed.aborted) downloadReviewReport(original.name, revised.name, report);
    } catch (error) {
      if (!completed.aborted) setExportError(error instanceof Error ? error.message : 'Could not save the PDF review report.');
    } finally {
      if (!completed.aborted) setSaving(false);
    }
  }
  return <>
    <p>Pages are paired by position. Pink marks changed pixels at preview resolution.</p>
    <p role="status">{status}</p>
    {completed && <div className="document-actions">
      <button type="button" onClick={saveReport} disabled={saving}>{saving ? 'Preparing report…' : 'Download review report'}</button>
      {exportError && <span role="alert">{exportError}</span>}
    </div>}
    {pages.map(p => <section key={p.number}><h2>Page {p.number} · {p.status}</h2>
      {p.url && <img style={{ maxWidth: '100%' }} src={p.url} alt={`Page ${p.number} visual comparison`} />}
    </section>)}
  </>;
}
