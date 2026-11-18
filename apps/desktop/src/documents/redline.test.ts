import { describe, expect, it } from 'vitest';
import { strFromU8, unzipSync } from 'fflate';
import { compareDocuments, trackedReport, type Span } from './redline';
import type { ParsedDocument } from './read';

const doc = (text: string): ParsedDocument => ({ format: 'docx', sections: new Map([['Document body', text]]) });
const accept = (spans: Span[]) => spans.filter(s => !s.removed).map(s => s.value).join('');
const reject = (spans: Span[]) => spans.filter(s => !s.added).map(s => s.value).join('');

describe('document redline alignment', () => {
  it('keeps the matching clause when an insertion precedes it and a repeated word is deleted', () => {
    const a = 'selected to provide advice to the client by a trustee';
    const b = 'eligible when selected to provide financial advice by a trustee';
    const [{ spans }] = compareDocuments(doc(a), doc(b));
    const kept = spans.filter(s => !s.added && !s.removed).map(s => s.value).join('');
    expect(kept).toContain('selected to provide');
    expect(kept).toContain('advice');
    expect(accept(spans)).toBe(b);
    expect(reject(spans)).toBe(a);
  });

  it.each([
    ['\nalpha\n\nend\n', 'alpha\n\nnew\nend'],
    ['alpha\nbeta', 'alpha\nbeta\n'],
    ['alpha\nbeta\n', 'alpha\nbeta'],
    ['A\tB\nC  D', 'A B\nC\u00a0D'],
    ['🧾 clause 甲\r\n\r\nWord\n', '🧾 clause 乙\nWord\t'],
    ['alpha\nalpha\nbeta\n\nbeta', 'beta\n\nalpha\nalpha\nbeta\n'],
    ['Section 1. Retain exactly thirty days.\nSection 2. Retain exactly sixty days.',
     'Section 1. Retain  exactly sixty days.\nSection 2. Retain exactly thirty days.'],
  ])('reconstructs original and revised text including every separator in the tracked Word report', (a, b) => {
    const sections = compareDocuments(doc(a), doc(b));
    const xml = strFromU8(unzipSync(trackedReport(sections))['word/document.xml']);
    const report = new DOMParser().parseFromString(xml, 'application/xml');
    function review(exclude: 'ins' | 'del') {
      const runs = Array.from(report.getElementsByTagNameNS('*', 'r'));
      return runs.filter(run => run.parentElement?.localName !== exclude).flatMap(run =>
        Array.from(run.children, piece => {
          if (piece.localName === 'br') return '\n';
          if (piece.localName === 'tab') return '\t';
          return piece.textContent;
        })).join('');
    }
    expect(review('ins')).toBe(a);
    expect(review('del')).toBe(b);
  });

  it('detects a moved paragraph even when paragraphs share the same opening', () => {
    const lines = ['pay', 'disclose', 'retain'].map(verb => 'The supplier shall ' + verb + ' the amount.');
    const revised = [lines[1], lines[2], lines[0]].join('\n');
    const [{ spans, moves }] = compareDocuments(doc(lines.join('\n')), doc(revised));
    expect(moves).toEqual([{ text: lines[0], from: 1, to: 3 }]);
    expect(accept(spans)).toBe(revised);
    expect(reject(spans)).toBe(lines.join('\n'));
  });
});
