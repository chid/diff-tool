import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { comparePdfPages } from './pdfVisual';
import { downloadReviewReport } from './reviewReport';
import PdfVisualReview from './PdfVisualReview';
import { visualReport } from './visualReport';

vi.mock('./pdfVisual', () => ({ comparePdfPages: vi.fn() }));
vi.mock('./reviewReport', () => ({ downloadReviewReport: vi.fn() }));

const original = new File([], 'original.pdf');
const revised = new File([], 'revised.pdf');
const png = new Blob([new Uint8Array([137, 80, 78, 71])], { type: 'image/png' });
const reportButton = () => screen.queryByRole('button', { name: 'Download review report' });

beforeEach(() => {
  vi.stubGlobal('URL', class extends URL {
    static createObjectURL = vi.fn(() => 'blob:test');
    static revokeObjectURL = vi.fn();
  });
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

it('embeds actual preview bytes and reports positional counts and the resolution limit', async () => {
  const signal = new AbortController().signal;
  const pages = [{ number: 1, status: 'Unchanged' }, { number: 2, status: 'Changed', preview: png }];
  for (const status of ['Added', 'Removed']) {
    const report = await visualReport([...pages, { number: 3, status, preview: png }], signal);
    const balance = status === 'Added' ? '1 added, 0 removed' : '0 added, 1 removed';
    expect(report.summary).toBe(`3 page positions compared. 1 changed, ${balance} and 1 unchanged.`);
    expect(report.items.map(i => i.title)).toEqual(['Page 2 · Changed', `Page 3 · ${status}`]);
    expect(report.items.map(i => i.image)).toEqual(['data:image/png;base64,iVBORw==', 'data:image/png;base64,iVBORw==']);
    expect(report.items[1].description).toContain(status === 'Added' ? 'Only the revision' : 'Only the original');
    expect(report.notes.join(' ')).toMatch(/paired by position.*1,400 pixels.*text review/s);
  }
});

it('summarizes all matching pages without adding rows of change details', async () => {
  const pages = Array.from({ length: 30 }, (_, index) => ({ number: index + 1, status: 'Unchanged' }));
  const report = await visualReport(pages, new AbortController().signal);
  expect(report.summary).toBe('All page positions matched at preview resolution. 30 pages unchanged.');
  expect(report.items).toEqual([]);
});

it('keeps a partial or failed comparison ineligible for export and reports missing previews', async () => {
  let fail!: (error: Error) => void;
  vi.mocked(comparePdfPages).mockImplementation((_left, _right, _signal, append) => {
    append({ number: 1, status: 'Changed', preview: png });
    return new Promise((_resolve, reject) => { fail = reject; });
  });
  const ui = render(<PdfVisualReview original={original} revised={revised} />);
  expect(screen.getByText('Page 1 · Changed')).toBeVisible();
  expect(reportButton()).toBeNull();
  await act(async () => { fail(new Error('Could not render page 2')); });
  expect(screen.getByRole('status')).toHaveTextContent('Could not render page 2');
  expect(reportButton()).toBeNull();

  vi.mocked(comparePdfPages).mockImplementationOnce(async (_left, _right, _signal, append) => {
    append({ number: 1, status: 'Added' });
  });
  ui.rerender(<PdfVisualReview original={original} revised={new File([], 'third.pdf')} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Download review report' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('The preview for page 1 is unavailable.');
  expect(reportButton()).toBeEnabled();
  expect(downloadReviewReport).not.toHaveBeenCalled();
});

it('discards an export if a file is replaced while a preview is being embedded', async () => {
  vi.mocked(comparePdfPages).mockImplementation(async (_left, _right, _signal, append) => {
    append({ number: 1, status: 'Changed', preview: png });
  });
  const read = FileReader.prototype.readAsDataURL;
  let finish!: () => Promise<void>;
  vi.spyOn(FileReader.prototype, 'readAsDataURL').mockImplementationOnce(function (this: FileReader, blob) {
    finish = () => new Promise(resolve => {
      this.addEventListener('loadend', () => resolve(), { once: true });
      read.call(this, blob);
    });
  });
  const ui = render(<PdfVisualReview original={original} revised={revised} />);
  fireEvent.click(await screen.findByRole('button', { name: 'Download review report' }));
  expect(screen.getByRole('button', { name: 'Preparing report…' })).toBeDisabled();
  const replacement = new File([], 'revised.pdf');
  ui.rerender(<PdfVisualReview original={original} revised={replacement} />);
  await act(finish);
  expect(downloadReviewReport).not.toHaveBeenCalled();
  fireEvent.click(await screen.findByRole('button', { name: 'Download review report' }));
  await vi.waitFor(() => expect(downloadReviewReport).toHaveBeenCalledOnce());
});
