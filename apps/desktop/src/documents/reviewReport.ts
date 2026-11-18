import reportStyles from './report.css?inline';

export interface ReviewReport {
  mode: string;
  summary: string;
  highlights?: string[];
  notes: string[];
  items: {
    title: string;
    description?: string;
    before?: string;
    after?: string;
    image?: string;
  }[];
}

function reportHtml(original: string, revised: string, report: ReviewReport) {
  const doc = document.implementation.createHTMLDocument('Document comparison report');
  doc.documentElement.lang = 'en';
  function add<K extends keyof HTMLElementTagNameMap>(parent: Element, tag: K, text?: string) {
    const element = doc.createElement(tag);
    if (text !== undefined) element.textContent = text;
    parent.append(element);
    return element;
  }

  const charset = doc.createElement('meta');
  charset.setAttribute('charset', 'utf-8');
  doc.head.prepend(charset);
  const policy = add(doc.head, 'meta');
  policy.httpEquiv = 'Content-Security-Policy';
  policy.content = "default-src 'none'; style-src 'unsafe-inline'; img-src data:; base-uri 'none'; form-action 'none'";
  const viewport = add(doc.head, 'meta');
  viewport.name = 'viewport';
  viewport.content = 'width=device-width, initial-scale=1';
  add(doc.head, 'style', reportStyles);

  const main = add(doc.body, 'main');
  const header = add(main, 'header');
  add(header, 'h1', 'Document comparison report');
  const files = add(header, 'dl');
  for (const [label, value] of [['Original', original], ['Revised', revised], ['Comparison', report.mode]]) {
    add(files, 'dt', label);
    add(files, 'dd', value);
  }
  add(main, 'h2', 'Summary');
  add(main, 'p', report.summary);
  if (report.highlights?.length) {
    const list = add(main, 'ul');
    for (const text of report.highlights) add(list, 'li', text);
  }
  if (report.items.length) add(main, 'h2', 'Change details');
  for (const item of report.items) {
    const section = add(main, 'section');
    add(section, 'h3', item.title);
    if (item.description) add(section, 'p', item.description);
    for (const [label, passage] of [['Before', item.before], ['After', item.after]] as const) {
      if (passage === undefined) continue;
      const quote = add(section, 'div');
      quote.className = label.toLowerCase();
      add(quote, 'strong', label);
      add(quote, 'blockquote', passage);
    }
    if (item.image?.startsWith('data:image/png;base64,')) {
      const image = add(section, 'img');
      image.alt = item.title;
      image.src = item.image;
    }
  }
  if (report.notes.length) {
    add(main, 'h2', 'Comparison notes');
    const notes = add(main, 'ul');
    for (const note of report.notes) add(notes, 'li', note);
  }
  return '<!doctype html>\n' + doc.documentElement.outerHTML;
}

export function downloadReviewReport(original: string, revised: string, report: ReviewReport) {
  const file = new Blob([reportHtml(original, revised, report)], { type: 'text/html;charset=utf-8' });
  const url = URL.createObjectURL(file);
  const stem = revised.replace(/\.[^.]+$/, '').replace(/[^\p{L}\p{N}_-]+/gu, '-').slice(0, 80);
  const link = document.createElement('a');
  link.href = url;
  link.download = (stem || 'comparison') + '-review-report.html';
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}
