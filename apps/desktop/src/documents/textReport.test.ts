import { describe, expect, it } from 'vitest';
import { compareDocuments, type Section } from './redline';
import { textReport } from './textReport';

const body = (value: string) => ({ format: 'docx' as const, sections: new Map([['Document body', value]]) });
const report = (sections: Section[]) => textReport(sections, [], 'original.docx', 'revision.docx');

describe('text comparison reports', () => {
  it('quotes both complete payment and delivery terms and describes a relocated clause once', () => {
    const packing = 'Place the order number on every crate.';
    const original = ['Orders', packing, 'Confirm the order.', 'Delivery', 'Deliver on Wednesday mornings.', 'Payment', 'We pay within 30 days of delivery.'];
    const revised = ['Orders', 'Confirm the order.', 'Delivery', 'Deliver on Friday mornings.', packing, 'Payment', 'We pay within 14 days of delivery.'];
    const result = report(compareDocuments(body(original.join('\n')), body(revised.join('\n'))));
    expect(result.items.map(item => [item.before, item.after])).toEqual([
      ['Deliver on Wednesday mornings.', 'Deliver on Friday mornings.'],
      ['We pay within 30 days of delivery.', 'We pay within 14 days of delivery.'],
      [undefined, packing],
    ]);
    expect(result.highlights?.join(' ')).toContain('“30 days” to “14 days”');
    expect(result.highlights?.join(' ')).toContain('Paragraph 2 moved to paragraph 5');
  });

  it('joins nearby edits into one truthful quotation for each version', () => {
    const a = 'For pickup, 12 cafés will share 240 bags today.';
    const b = 'For pickup, 18 cafés will share 360 bags today.';
    const result = report(compareDocuments(body(a), body(b)));
    expect(result.items.map(item => [item.before, item.after])).toEqual([[a, b]]);
    expect(result.items[0].description).toContain('12 cafés will share 240');
    expect(result.items[0].description).toContain('18 cafés will share 360');
  });

  it('represents different slides and embedded media in the highlights before repeating a section', () => {
    const slide = (label: string): Section => ({
      label, status: 'modified', spans: ['black', 'white', 'green', 'yellow', 'red'].flatMap(color => [
        { value: 'Paint it ' }, { value: color, removed: true }, { value: 'blue', added: true }, { value: '.\n' },
      ]),
    });
    const media: Section = { label: 'ppt/media/photo.png', status: 'modified', spans: [
      { value: 'Original media (90 bytes)', removed: true }, { value: 'Revised media (90 bytes)', added: true },
    ] };
    const result = report([slide('Slide 1'), slide('Slide 2'), media]);
    expect(result.highlights?.slice(0, 3).map(value => value.split(' — ')[0])).toEqual(['Slide 1', 'Slide 2', media.label]);
    expect(result.items).toHaveLength(11);
    expect(result.items.at(-1)?.description).toContain('different bytes');
  });

  it('distinguishes table structure, empty added sections, unchanged text, and empty input', () => {
    const structure: Section = { label: 'Table 2 row 1 structure', status: 'modified', spans: [
      { value: 'cell' }, { value: ' | cell', added: true },
    ] };
    const empty: Section = { label: 'Slide 4', status: 'added', spans: [{ value: '', added: true }] };
    expect(report([structure, empty]).items.map(item => item.description)).toEqual([
      'The row changed from 1 cell to 2 cells.', 'This section was added. It contains no visible text.',
    ]);
    const notes = ['Revised used English OCR on page(s) 1.'];
    const same = textReport([{ label: 'Document body', status: 'unchanged', spans: [{ value: 'OK' }] }], notes, 'a.PDF', 'b.docx');
    expect(same.items).toEqual([]);
    expect(same.notes[0]).toBe(notes[0]);
    expect(same.summary).toContain('No differences');
    expect(same.notes.join(' ')).toContain('Drawings and page layout require visual review');
    expect(report([]).summary).toContain('no comparable sections');
  });

  it('names the limits of the selected format even when a pair has no detected changes', () => {
    for (const [format, scope] of [['PPTX', 'Slide layout'], ['XLSX', 'Recalculation'], ['DOCX', 'pagination']]) {
      const result = textReport([{ label: 'Example', status: 'unchanged', spans: [{ value: 'same' }] }], [], 'original.' + format, 'revised.' + format);
      expect(result.notes.join(' ')).toContain(scope);
    }
  });

  it('clips both ends of long context while keeping the complete changed passage and both ends of its preview', () => {
    const original = 'old start ' + 'long old content '.repeat(40) + 'old ending';
    const revised = 'new start ' + 'long new content '.repeat(40) + 'new ending';
    const result = report([{ label: 'Document body', status: 'modified', spans: [
      { value: 'Opening boundary ' + 'left context '.repeat(25) },
      { removed: true, value: original }, { added: true, value: revised },
      { value: ' right context'.repeat(25) + ' closing boundary' },
    ] }]);
    const item = result.items[0];
    expect(item.before).toMatch(/^….*…$/);
    expect(item.after).toMatch(/^….*…$/);
    expect(item.before).toContain(original);
    expect(item.after).toContain(revised);
    expect(item.description).toMatch(/old start .*….*old ending.*new start .*….*new ending/);
  });
});
