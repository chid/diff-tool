import { diffArrays, diffWordsWithSpace } from 'diff';
import { strToU8, zipSync } from 'fflate';
import type { ParsedDocument } from './read';

export interface Span { value: string; added?: boolean; removed?: boolean }
export interface Section {
  label: string;
  spans: Span[];
  moves?: { text: string; from: number; to: number }[];
  status: 'unchanged' | 'modified' | 'added' | 'removed';
}

function movedParagraphs(original: string, revised: string) {
  const before = original.split('\n'), after = revised.split('\n');
  if (before.length < 2 || after.length < 2) return [];
  const edits = diffArrays(before, after, { timeout: 250 });
  if (!edits) return [];
  function positions(lines: string[]) {
    const index = new Map<string, number>();
    lines.forEach((line, i) => index.set(line, index.has(line) ? -1 : i + 1));
    return index;
  }
  const oldPositions = positions(before), newPositions = positions(after);
  const removed = new Set(edits.filter(c => c.removed).flatMap(c => c.value));
  return edits.filter(c => c.added).flatMap(c => c.value).flatMap(text => {
    const from = oldPositions.get(text), to = newPositions.get(text);
    return text.trim() && removed.has(text) && from && from > 0 && to && to > 0 && from !== to ? [{ text, from, to }] : [];
  });
}

export function compareDocuments(left: ParsedDocument, right: ParsedDocument): Section[] {
  const structured = (d: ParsedDocument) => d.format === 'xlsx' || d.format === 'pptx';
  if (left.format !== right.format && (structured(left) || structured(right))) {
    throw new Error('Compare a workbook with another workbook, or a presentation with another presentation.');
  }
  const pdf = left.format === 'pdf' || right.format === 'pdf';
  // PDF line wraps are page layout. Compare the reading order of the body across formats.
  const labels = pdf ? ['Document body'] : new Set([...left.sections.keys(), ...right.sections.keys()]);
  const result: Section[] = [];
  for (const label of labels) {
    const normalize = (s: string | undefined) => pdf ? s?.replace(/\s+/gu, ' ').trim() : s;
    const a = normalize(left.sections.get(label));
    const b = normalize(right.sections.get(label));
    if (a === b) result.push({ label, status: 'unchanged', spans: [{ value: a ?? '' }] });
    else if (a === undefined) result.push({ label, status: 'added', spans: [{ value: b!, added: true }] });
    else if (b === undefined) result.push({ label, status: 'removed', spans: [{ value: a, removed: true }] });
    else {
      const spans = diffWordsWithSpace(a, b, { timeout: 1500 });
      // Whole-section replacement still reconstructs both texts if detailed alignment times out.
      const moves = label === 'Document body' && left.format === 'docx' && right.format === 'docx'
        ? movedParagraphs(a, b) : undefined;
      result.push({ label, status: 'modified', spans: spans ?? [{ value: a, removed: true }, { value: b, added: true }], moves });
    }
  }
  if (left.format === right.format && left.media && right.media) {
    const paths = new Set([...left.media.keys(), ...right.media.keys()]);
    for (const path of paths) {
      const before = left.media.get(path), after = right.media.get(path);
      if (before && after && before.length === after.length && before.every((byte, i) => byte === after[i])) continue;
      const spans: Span[] = [];
      if (before) spans.push({ removed: true, value: `Original media (${before.length} bytes)` });
      if (after) spans.push({ added: true, value: `Revised media (${after.length} bytes)` });
      result.push({ label: path, status: before && after ? 'modified' : after ? 'added' : 'removed', spans });
    }
  }
  return result;
}

const escapeXml = (s: string) => s.replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]!);

export function trackedReport(sections: Section[]) {
  let id = 0;
  function runs(span: Span) {
    const tag = span.removed ? 'w:delText' : 'w:t';
    const fragments = span.value.split(/(\n|\t)/).map(piece => {
      if (piece === '\n') return '<w:br/>';
      if (piece === '\t') return '<w:tab/>';
      return `<${tag} xml:space="preserve">${escapeXml(piece.replace(/[\u0000-\u0008\u000b-\u000c\u000e-\u001f]/g, '\uFFFD'))}</${tag}>`;
    }).join('');
    const run = '<w:r>' + fragments + '</w:r>';
    if (!span.added && !span.removed) return run;
    const revision = span.added ? 'w:ins' : 'w:del';
    return `<${revision} w:id="${++id}" w:author="Diff Tool">${run}</${revision}>`;
  }
  const paragraphs = sections.map(section => {
    const title = section.label === 'Document body' ? '' : `<w:p>${runs({ value: section.label })}</w:p>`;
    return title + '<w:p>' + section.spans.map(runs).join('') + '</w:p>';
  }).join('');
  const word = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main';
  const opc = 'http://schemas.openxmlformats.org/package/2006';
  return zipSync({
    '[Content_Types].xml': strToU8(`<Types xmlns="${opc}/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>`),
    '_rels/.rels': strToU8(`<Relationships xmlns="${opc}/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>`),
    'word/document.xml': strToU8(`<w:document xmlns:w="${word}"><w:body>${paragraphs}<w:sectPr/></w:body></w:document>`),
  });
}
