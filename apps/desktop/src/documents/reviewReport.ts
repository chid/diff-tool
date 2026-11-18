import reportStyles from './report.css?inline';
import type { GovernanceCase } from './governance';

export interface ReviewReport {
  mode: string;
  summary: string;
  governance?: GovernanceCase;
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

export function governanceReport(brief: GovernanceCase): ReviewReport {
  return { mode: 'AI governance review', summary: brief.finding, governance: brief, notes: [], items: [] };
}

function add<K extends keyof HTMLElementTagNameMap>(parent: Element, tag: K, text?: string) {
  const element = parent.ownerDocument.createElement(tag);
  if (text !== undefined) element.textContent = text;
  parent.append(element);
  return element;
}

function sourceLink(parent: Element, label: string, url: string) {
  const link = add(parent, 'a', label);
  link.href = url;
  link.target = '_blank';
  link.rel = 'noopener noreferrer';
}

function writeGovernanceBrief(main: HTMLElement, brief: GovernanceCase) {
  main.className = 'governance-brief';
  const header = add(main, 'header');
  add(header, 'p', `${brief.company} · ${brief.document}`).className = 'document-name';
  add(header, 'h1', brief.title);
  add(header, 'p', brief.scope).className = 'document-scope';
  add(main, 'h2', 'Finding');
  add(main, 'p', brief.finding).className = 'finding';

  add(main, 'h2', 'Source comparison');
  for (const [index, edition] of [brief.before, brief.after].entries()) {
    const article = add(main, 'article');
    add(article, 'h3', index === 0 ? 'Earlier guidance' : 'Later guidance');
    sourceLink(article, `${edition.label} (${edition.date})`, edition.url);
    add(article, 'blockquote', edition.excerpt).cite = edition.url;
    add(article, 'p', 'Analyst note').className = 'analyst-label';
    add(article, 'p', edition.text).className = 'analyst-note';
  }

  const followUp = add(main, 'section');
  followUp.className = 'follow-up';
  add(followUp, 'h2', 'Client follow-up');
  add(followUp, 'p', brief.question);
  if (brief.reference) sourceLink(add(main, 'footer'), brief.reference.label, brief.reference.url);
}

function reportHtml(original: string, revised: string, report: ReviewReport) {
  const doc = document.implementation.createHTMLDocument(report.governance?.title ?? 'Document comparison report');
  doc.documentElement.lang = 'en';

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
  if (report.governance) writeGovernanceBrief(main, report.governance);
  else writeComparison(main, original, revised, report);
  return '<!doctype html>\n' + doc.documentElement.outerHTML;
}

function writeComparison(main: HTMLElement, original: string, revised: string, report: ReviewReport) {
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
