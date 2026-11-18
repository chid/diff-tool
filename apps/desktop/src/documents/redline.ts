import { diffArrays, diffWords, diffWordsWithSpace, type ChangeObject } from 'diff';
import { strToU8, zipSync } from 'fflate';
import type { ParsedDocument } from './read';

export interface Span { value: string; added?: boolean; removed?: boolean }
export interface Section {
  label: string;
  spans: Span[];
  moves?: { text: string; from: number; to: number }[];
  status: 'unchanged' | 'modified' | 'added' | 'removed';
}

// diffWords aligns words and punctuation without treating a common space as a match.
// It can normalize whitespace. Only use those spans when accepting and rejecting
// every edit would reproduce both inputs exactly.
function reconstructs(spans: Span[], before: string, after: string) {
  let oldOffset = 0, newOffset = 0;
  for (const span of spans) {
    if (!span.added) {
      if (!before.startsWith(span.value, oldOffset)) return false;
      oldOffset += span.value.length;
    }
    if (!span.removed) {
      if (!after.startsWith(span.value, newOffset)) return false;
      newOffset += span.value.length;
    }
  }
  return oldOffset === before.length && newOffset === after.length;
}

function wordSpans(before: string, after: string, deadline: number): Span[] {
  if (before === after) return before ? [{ value: before }] : [];
  if (!before) return [{ value: after, added: true }];
  if (!after) return [{ value: before, removed: true }];
  const remaining = () => deadline - performance.now();
  if (remaining() > 0) {
    const spans = diffWords(before, after, { timeout: remaining() });
    if (spans && reconstructs(spans, before, after)) return spans;
    if (spans && remaining() > 0) {
      const exact = diffWordsWithSpace(before, after, { timeout: remaining() });
      if (exact) return exact;
    }
  }
  return [{ value: before, removed: true }, { value: after, added: true }];
}

function movedParagraphs(before: string[], after: string[], edits: ChangeObject<string[]>[]) {
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

function canPairInOrder(before: string[], after: string[]) {
  if (before.length !== after.length) return false;
  const changed = new Set<string>();
  for (let i = 0; i < before.length; i++) {
    const a = before[i], b = after[i];
    if (a === b) continue;
    // Only pair edited paragraphs with substantial shared context at an edge.
    // Otherwise use the paragraph diff, which can align insertions and moves.
    const start = a.slice(0, 16), end = a.slice(-16);
    if (!(start.trim().length >= 12 && b.startsWith(start)) &&
        !(end.trim().length >= 12 && b.endsWith(end))) return false;
    changed.add(a);
  }
  // An exact match at another position takes precedence over similar context.
  return !after.some((text, i) => text !== before[i] && changed.has(text));
}

function bodyChanges(original: string, revised: string, deadline: number) {
  const before = original.split('\n'), after = revised.split('\n');
  if (before.length === 1 || after.length === 1) {
    return { spans: wordSpans(original, revised, deadline), moves: [] };
  }
  const spans: Span[] = [];
  function append(part: Span) {
    if (!part.value) return;
    const last = spans.at(-1);
    if (last && !!last.added === !!part.added && !!last.removed === !!part.removed) last.value += part.value;
    else spans.push({ value: part.value, added: part.added, removed: part.removed });
  }
  if (canPairInOrder(before, after)) {
    for (let i = 0; i < before.length; i++) {
      if (i) append({ value: '\n' });
      for (const part of wordSpans(before[i], after[i], deadline)) append(part);
    }
    return { spans, moves: [] };
  }

  const edits = diffArrays(before, after, { timeout: 250 });
  if (!edits) return { spans: wordSpans(original, revised, deadline), moves: [] };
  // Exact paragraph blocks anchor the word comparison. Include the intervening
  // separators in the gaps so leading, blank and trailing lines survive export.
  let oldOffset = 0, newOffset = 0, oldEnd = 0, newEnd = 0;
  function gap(oldStart: number, newStart: number) {
    for (const part of wordSpans(original.slice(oldEnd, oldStart), revised.slice(newEnd, newStart), deadline)) append(part);
  }
  for (const edit of edits) {
    const block = edit.value.join('\n');
    if (!edit.added && !edit.removed) {
      gap(oldOffset, newOffset);
      append({ value: block });
      oldEnd = oldOffset + block.length;
      newEnd = newOffset + block.length;
    }
    if (!edit.added) oldOffset += block.length + 1;
    if (!edit.removed) newOffset += block.length + 1;
  }
  gap(original.length, revised.length);
  return { spans, moves: movedParagraphs(before, after, edits) };
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
      const deadline = performance.now() + 1500;
      const changes = label === 'Document body' && left.format === 'docx' && right.format === 'docx'
        ? bodyChanges(a, b, deadline) : { spans: wordSpans(a, b, deadline) };
      result.push({ label, status: 'modified', ...changes });
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

// A literal CR is normalized to LF by XML parsers, including inside w:t.
const escapeXml = (s: string) => s.replace(/[&<>\r]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '\r': '&#13;' })[c]!);

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
