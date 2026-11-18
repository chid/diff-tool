// Run with Node 22.18+ and Poppler. Scope and source are in docs/document-comparison.md.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { JSDOM } from 'jsdom';
import { compareDocuments } from '../apps/desktop/src/documents/redline.ts';

const pdf = process.argv[2];
const digest = createHash('sha256').update(readFileSync(pdf)).digest('hex');
const references = {
  f9a8139704db3bb7ddc583ea8a824e1b2f323965626663d2d3a5138d93ca6b44: {
    name: 'PTE 2020-02', rows: 76, exact: 63,
    deletions: { marked: 193, agreed: 180, extra: 0 }, insertions: { marked: 252, agreed: 239, extra: 0 },
  },
  '28bcd8d18e6ee5c526fc169819a73c2ff17a19348eb499c86a3ca1edd968d36d': {
    name: '2022-15702 and 2024-06059', rows: 55, exact: 41,
    deletions: { marked: 162, agreed: 145, extra: 0 }, insertions: { marked: 179, agreed: 159, extra: 3 },
  },
};
const reference = references[digest];
if (!reference) throw new Error('Expected one of the two PDFs linked in docs/document-comparison.md.');
const xml = execFileSync('pdftohtml', ['-i', '-xml', '-stdout', pdf], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
const document = new JSDOM(xml, { contentType: 'text/xml' }).window.document;
const colors = new Map([...document.querySelectorAll('fontspec')].map(el => [el.id, el.getAttribute('color')]));
const black = '#000000', red = '#ff0000', blue = '#0000ff';
const counts = () => ({ marked: 0, detected: 0, agreed: 0 });
const scores = { deletions: counts(), insertions: counts() };
let rows = 0, exact = 0;

// Count word positions on each side. A repeated word elsewhere cannot satisfy a mark.
function wordMarks(parts) {
  const text = parts.map(p => p.text).join('');
  const mask = parts.map(p => (p.marked ? '1' : '0').repeat(p.text.length)).join('');
  return [...text.matchAll(/[\p{L}\p{N}]+/gu)].map(m => mask.slice(m.index, m.index + m[0].length).includes('1'));
}
function score(reference, actual, bucket) {
  const expected = wordMarks(reference), observed = wordMarks(actual);
  if (expected.length !== observed.length) throw new Error('Word positions did not align');
  for (const [i, flag] of expected.entries()) {
    bucket.marked += Number(flag);
    bucket.detected += Number(observed[i]);
    bucket.agreed += Number(flag && observed[i]);
  }
  return expected.every((flag, i) => flag === observed[i]);
}

function evaluate(parts) {
  const present = new Set(parts.map(p => p.color));
  if (![red, blue, black].every(c => present.has(c)) || present.size !== 3) return;
  function side(omit) {
    // Colored runs in a PDF can abut without storing a space. Normalize to a
    // word stream so projecting out the opposite color cannot fuse two words.
    const words = parts.filter(p => p.color !== omit).flatMap(p =>
      [...p.text.matchAll(/[\p{L}\p{N}]+/gu)].map(m => ({ text: m[0], marked: p.color !== black })));
    return words.map((w, i) => ({ ...w, text: (i ? ' ' : '') + w.text }));
  }
  const old = side(blue), revised = side(red);
  if (![old, revised].every(s => wordMarks(s).includes(true))) return;
  const asDocument = parts => ({ format: 'docx', sections: new Map([['Document body', parts.map(p => p.text).join('')]]) });
  const [body] = compareDocuments(asDocument(old), asDocument(revised));
  const a = score(old, body.spans.filter(s => !s.added).map(s => ({ text: s.value, marked: s.removed })), scores.deletions);
  const b = score(revised, body.spans.filter(s => !s.removed).map(s => ({ text: s.value, marked: s.added })), scores.insertions);
  rows++;
  if (a && b) exact++;
}

// Poppler places smaller struck-out text about 2–3 pixels below the baseline
// of surrounding text. Exclude move-colored rows and the final legend.
for (const page of [...document.querySelectorAll('page')].slice(0, -1)) {
  let row = [], top = -100, lastLeft = -100;
  for (const el of page.querySelectorAll('text')) {
    const y = Number(el.getAttribute('top')), x = Number(el.getAttribute('left'));
    if (Math.abs(y - top) > 4 || x < lastLeft) {
      evaluate(row); row = []; top = y;
    }
    lastLeft = x;
    row.push({ text: el.textContent, color: colors.get(el.getAttribute('font')) });
  }
  evaluate(row);
}
if (!rows) throw new Error('No eligible marked rows found.');
console.log(JSON.stringify({ reference: reference.name, rows, exact, ...scores }, null, 2));

// Guard recall and extra marks separately on the same pinned word positions.
const regressed = Object.entries(scores).some(([side, score]) => {
  const expected = reference[side];
  return score.marked !== expected.marked || score.agreed < expected.agreed ||
    score.detected - score.agreed > expected.extra;
});
if (rows !== reference.rows || exact < reference.exact || regressed) {
  console.error('The population or word-mark agreement failed the published reference baseline.');
  process.exitCode = 1;
}
