import { lazy, Suspense, useEffect, useRef, useState } from 'react';
import { readDocument } from './read';
import { compareDocuments, trackedReport, type Section } from './redline';
import './review.css';

const PdfVisualReview = lazy(() => import('./PdfVisualReview'));

export type FilePair = { left?: File; right?: File };
type Result = { sections: Section[]; notes: string[] } | { error: string };

function saveReport(sections: Section[]) {
  const bytes = trackedReport(sections);
  const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = 'comparison-redline.docx';
  a.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export default function DocumentReview({ files }: { files: FilePair }) {
  const [finished, setFinished] = useState<{ files: FilePair; result: Result }>();
  const [showAll, setShowAll] = useState(false);
  const [pageMode, setPageMode] = useState(false);
  const reviewRef = useRef<HTMLElement>(null);
  const [position, setPosition] = useState(0);
  const pdfPair = [files.left, files.right].every(f => f && /\.pdf$/i.test(f.name));
  const visual = pageMode && pdfPair;
  useEffect(() => {
    setPosition(0);
    const controller = new AbortController();
    const { left, right } = files;
    if (left && right && !visual) {
      Promise.all([readDocument(left, controller.signal), readDocument(right, controller.signal)])
        .then(([a, b]) => ({
          sections: compareDocuments(a, b),
          notes: [a, b].flatMap((doc, i) => doc.ocrPages?.length
            ? [`${i === 0 ? 'Original' : 'Revised'} used English OCR on page(s) ${doc.ocrPages.join(', ')}. Check the recognized text against the scan.`] : []),
        }))
        .catch((e: unknown) => ({ error: e instanceof Error ? e.message : String(e) }))
        .then(result => { if (!controller.signal.aborted) setFinished({ files, result }); });
    }
    return () => controller.abort();
  }, [files, visual]);

  const result = finished?.files === files ? finished.result : null;
  const sections = result && 'sections' in result ? result.sections : undefined;
  const changes = sections?.filter(s => s.status !== 'unchanged') ?? [];
  const visible = showAll ? sections ?? [] : changes;
  const isEditStart = (s: Section, i: number) => {
    const span = s.spans[i], prev = s.spans[i - 1];
    return (span.added || span.removed) && !prev?.added && !prev?.removed;
  };
  const totalEdits = changes.reduce((n, s) => n + s.spans.filter((_, i) => isEditStart(s, i)).length, 0);
  function navigate(step: number) {
    const next = position ? (position - 1 + step + totalEdits) % totalEdits : (step > 0 ? 0 : totalEdits - 1);
    const element = reviewRef.current?.querySelectorAll<HTMLElement>('[data-edit-start]')[next];
    element?.focus({ preventScroll: true });
    element?.scrollIntoView({ block: 'center' });
    setPosition(next + 1);
  }
  return <article ref={reviewRef} className="document-review" aria-label="Document comparison">
    <p>Choose the original and revised files above, or drop them together. Files are read on this device.</p>
    <p className="document-scope">Compare Word and PDF text (English OCR for scanned pages), Excel cells and formulas, or PowerPoint slide text. Word review includes direct body formatting, table structure, and uniquely matched paragraph moves. Changed Office media files are listed. Page layout and chart rendering require visual review. Word-to-PDF comparison uses body text.</p>
    {pdfPair && <label><input type="checkbox" checked={pageMode} onChange={e => setPageMode(e.target.checked)} /> Compare rendered PDF pages</label>}
    {visual ? <Suspense fallback={<p>Loading page review…</p>}><PdfVisualReview original={files.left!} revised={files.right!} /></Suspense> :
      result && 'error' in result ? <p role="alert">{result.error}</p> :
      !files.left || !files.right ? <p role="status">Choose {files.left ? 'the revised file' : files.right ? 'the original file' : 'two files'} to begin.</p> :
      !sections ? <p role="status">Reading documents…</p> : <>
        {result && 'notes' in result && result.notes.map(note => <p key={note}>{note}</p>)}
        <p className="print-doc-names">{files.left.name} → {files.right.name}</p>
        <div className="document-actions">
          <strong role="status">{changes.length ? `${changes.length} changed section${changes.length === 1 ? '' : 's'}` : 'No differences in the compared text'}</strong>
          <label><input type="checkbox" checked={showAll} onChange={e => setShowAll(e.target.checked)} /> Include unchanged sections</label>
          {totalEdits > 0 && <div role="group" aria-label="Navigate edits">
            <button type="button" onClick={() => navigate(-1)}>Previous edit</button>
            <span aria-live="polite">{position ? ` ${position} / ${totalEdits} ` : ' '}</span>
            <button type="button" onClick={() => navigate(1)}>Next edit</button>
          </div>}
          <button type="button" onClick={() => window.print()}>Print / Save PDF</button>
          <button type="button" onClick={() => saveReport(sections)}>Download tracked Word report</button>
        </div>
        {changes.length > 1 && <nav aria-label="Changed sections"><select aria-label="Jump to change" defaultValue="" onChange={e => {
          document.getElementById('document-section-' + e.target.value)?.scrollIntoView({ block: 'center' });
          e.target.value = '';
        }}><option value="" disabled>Jump to change…</option>{changes.map((s, i) => <option key={s.label} value={encodeURIComponent(s.label)}>{i + 1}. {s.label}</option>)}</select></nav>}
        {visible.map(s => <section key={s.label} id={'document-section-' + encodeURIComponent(s.label)}>
          <h2>{s.label} <small>{s.status}</small></h2>
          {!!s.moves?.length && <ul aria-label="Moved paragraphs">{s.moves.map(m =>
            <li key={m.from}>Paragraph {m.from} moved to {m.to} · {m.text}</li>)}</ul>}
          <div className="document-prose">{s.spans.map((span, i) => {
            const Tag = span.added ? 'ins' : span.removed ? 'del' : 'span';
            const anchor = isEditStart(s, i);
            return <Tag key={i} tabIndex={anchor ? -1 : undefined} data-edit-start={anchor || undefined}>{span.value}</Tag>;
          })}</div>
        </section>)}
      </>}
  </article>;
}
