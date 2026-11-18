import { useEffect, useState } from 'react';
import { comparePdfPages } from './pdfVisual';

export default function PdfVisualReview({ original, revised }: { original: File; revised: File }) {
  const [pages, setPages] = useState<{ number: number; status: string; url?: string }[]>([]);
  const [status, setStatus] = useState('Comparing pages…');
  useEffect(() => {
    const controller = new AbortController();
    const urls: string[] = [];
    setPages([]);
    setStatus('Comparing pages…');
    comparePdfPages(original, revised, controller.signal, p => {
      if (controller.signal.aborted) return;
      const url = p.preview ? URL.createObjectURL(p.preview) : undefined;
      if (url) urls.push(url);
      setPages(prev => [...prev, { number: p.number, status: p.status, url }]);
    }).then(() => { if (!controller.signal.aborted) setStatus('Page comparison complete'); })
      .catch((e: unknown) => { if (!controller.signal.aborted) setStatus(e instanceof Error ? e.message : String(e)); });
    return () => { controller.abort(); urls.forEach(url => URL.revokeObjectURL(url)); };
  }, [original, revised]);
  return <>
    <p>Pages are paired by position. Pink marks changed pixels at preview resolution.</p>
    <p role="status">{status}</p>
    {pages.map(p => <section key={p.number}><h2>Page {p.number} · {p.status}</h2>
      {p.url && <img style={{ maxWidth: '100%' }} src={p.url} alt={`Page ${p.number} visual comparison`} />}
    </section>)}
  </>;
}
