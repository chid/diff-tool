import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SettingsProvider } from '../SettingsContext';
import { NativeDiffRenderer, splitIntraText } from '../NativeDiffRenderer';
import type { DiffResult, IntraSpan } from '../../types';

const STORAGE_KEY = 'diff-tool-settings';

// ─── splitIntraText tests (pure function) ────────────────────────────────

describe('splitIntraText', () => {
  describe('empty / edge cases', () => {
    it('returns whole text unhighlighted when spans array is empty', () => {
      const result = splitIntraText('hello world', []);
      expect(result).toEqual([{ text: 'hello world', highlight: false }]);
    });

    it('handles empty text string', () => {
      const result = splitIntraText('', [{ start: 0, len: 5 }]);
      expect(result).toEqual([{ text: '', highlight: false }]);
    });

    it('handles empty text with empty spans', () => {
      const result = splitIntraText('', []);
      expect(result).toEqual([{ text: '', highlight: false }]);
    });

    it('skips zero-length spans without splitting', () => {
      // A zero-length span at byte 2 of "abcdef" should not split the text
      const spans: IntraSpan[] = [{ start: 2, len: 0 }];
      const result = splitIntraText('abcdef', spans);
      expect(result).toEqual([{ text: 'abcdef', highlight: false }]);
    });

    it('skips zero-length span between valid spans', () => {
      // Zero-length span between two real spans should not affect splitting
      const spans: IntraSpan[] = [
        { start: 0, len: 1 },
        { start: 1, len: 0 },
        { start: 2, len: 1 },
      ];
      const result = splitIntraText('a.b', spans);
      expect(result).toEqual([
        { text: 'a', highlight: true },
        { text: '.', highlight: false },
        { text: 'b', highlight: true },
      ]);
    });
  });

  describe('ASCII spans', () => {
    it('highlights a span at the start of the string', () => {
      const spans: IntraSpan[] = [{ start: 0, len: 5 }];
      const result = splitIntraText('hello world', spans);
      expect(result).toEqual([
        { text: 'hello', highlight: true },
        { text: ' world', highlight: false },
      ]);
    });

    it('highlights a span in the middle of the string', () => {
      const spans: IntraSpan[] = [{ start: 2, len: 2 }];
      const result = splitIntraText('abcdef', spans);
      expect(result).toEqual([
        { text: 'ab', highlight: false },
        { text: 'cd', highlight: true },
        { text: 'ef', highlight: false },
      ]);
    });

    it('highlights a span at the end of the string', () => {
      const spans: IntraSpan[] = [{ start: 3, len: 2 }];
      const result = splitIntraText('world', spans);
      expect(result).toEqual([
        { text: 'wor', highlight: false },
        { text: 'ld', highlight: true },
      ]);
    });

    it('handles multiple spans correctly', () => {
      const spans: IntraSpan[] = [
        { start: 1, len: 1 },
        { start: 3, len: 1 },
      ];
      const result = splitIntraText('aXbYc', spans);
      expect(result).toEqual([
        { text: 'a', highlight: false },
        { text: 'X', highlight: true },
        { text: 'b', highlight: false },
        { text: 'Y', highlight: true },
        { text: 'c', highlight: false },
      ]);
    });

    it('highlights the entire string when span covers all bytes', () => {
      const spans: IntraSpan[] = [{ start: 0, len: 5 }];
      const result = splitIntraText('hello', spans);
      expect(result).toEqual([{ text: 'hello', highlight: true }]);
    });

    it('handles adjacent spans (back to back changes)', () => {
      const spans: IntraSpan[] = [
        { start: 0, len: 3 },
        { start: 3, len: 3 },
      ];
      const result = splitIntraText('oldnew', spans);
      expect(result).toEqual([
        { text: 'old', highlight: true },
        { text: 'new', highlight: true },
      ]);
    });

    it('returns unhighlighted text when spans are out of range', () => {
      const spans: IntraSpan[] = [{ start: 100, len: 5 }];
      const result = splitIntraText('short', spans);
      expect(result).toEqual([{ text: 'short', highlight: false }]);
    });
  });

  describe('Unicode (multi-byte UTF-8)', () => {
    it('handles 2-byte UTF-8 characters like \u00e9', () => {
      // "caf\u00e9" = c(1) a(1) f(1) \u00e9(2) = 5 bytes total
      // Span [3,2] covers the "\u00e9" (byte offsets from Rust)
      const spans: IntraSpan[] = [{ start: 3, len: 2 }];
      const result = splitIntraText('caf\u00e9', spans);
      expect(result).toEqual([
        { text: 'caf', highlight: false },
        { text: '\u00e9', highlight: true },
      ]);
    });

    it('handles span starting at a multi-byte character boundary', () => {
      // "\u03b1\u03b2\u03b3" — each is 2 bytes = 6 bytes total
      // Span [2,4] covers "\u03b2\u03b3" (bytes 2..6)
      const spans: IntraSpan[] = [{ start: 2, len: 4 }];
      const result = splitIntraText('\u03b1\u03b2\u03b3', spans);
      expect(result).toEqual([
        { text: '\u03b1', highlight: false },
        { text: '\u03b2\u03b3', highlight: true },
      ]);
    });

    it('handles 3-byte UTF-8 characters like CJK', () => {
      // "a\u672cc" — a(1) \u672c(3) c(1) = 5 bytes
      // Span [1,3] covers "\u672c"
      const spans: IntraSpan[] = [{ start: 1, len: 3 }];
      const result = splitIntraText('a\u672cc', spans);
      expect(result).toEqual([
        { text: 'a', highlight: false },
        { text: '\u672c', highlight: true },
        { text: 'c', highlight: false },
      ]);
    });

    it('handles mixed ASCII and Unicode spans', () => {
      // "caf\u00e9 latte" — 12 bytes total
      // Span [3,2] covers "\u00e9"
      // Span [6,5] covers "latte"
      const spans: IntraSpan[] = [
        { start: 3, len: 2 },
        { start: 6, len: 5 },
      ];
      const result = splitIntraText('caf\u00e9 latte', spans);
      expect(result).toEqual([
        { text: 'caf', highlight: false },
        { text: '\u00e9', highlight: true },
        { text: ' ', highlight: false },
        { text: 'latte', highlight: true },
      ]);
    });
  });

  describe('surrogate pairs (emoji / non-BMP)', () => {
    // Note: jsdom may not perfectly preserve surrogate pair characters in
    // error messages, so these tests verify structural properties rather than
    // exact text equality.

    it('handles an emoji as a 4-byte UTF-8 span', () => {
      // "hi🎉bye" — h(1) i(1) 🎉(4) b(1) y(1) e(1) = 9 bytes
      // JS string "hi🎉bye" has .length = 6 (surrogate pair = 2 code units)
      // Span [2,4] covers the emoji (byte offsets 2..6, char indices 2..4)
      const text = 'hi\ud83c\udf89bye';
      const spans: IntraSpan[] = [{ start: 2, len: 4 }];
      const result = splitIntraText(text, spans);
      expect(result.length).toBe(3);
      expect(result[0].highlight).toBe(false); // "hi"
      expect(result[1].highlight).toBe(true);  // "🎉"
      expect(result[2].highlight).toBe(false); // "bye"
      // Verify the emoji is intact: result[1] should be a single surrogate pair
      expect(result[1].text.length).toBe(2); // surrogate pair = 2 JS chars
      expect(result[1].text.charCodeAt(0)).toBe(0xd83c);
      expect(result[1].text.charCodeAt(1)).toBe(0xdf89);
    });

    it('handles text starting with an emoji', () => {
      // "🌟star" — 🌟(4) s(1) t(1) a(1) r(1) = 8 bytes
      // Span [0,4] covers "🌟"
      const text = '\ud83c\udf1fstar';
      const spans: IntraSpan[] = [{ start: 0, len: 4 }];
      const result = splitIntraText(text, spans);
      expect(result.length).toBe(2);
      expect(result[0].highlight).toBe(true); // "🌟"
      expect(result[1].highlight).toBe(false); // "star"
      expect(result[1].text).toBe('star');
      // Verify the emoji: should be a surrogate pair
      expect(result[0].text.length).toBe(2);
      expect(result[0].text.charCodeAt(0)).toBe(0xd83c);
      expect(result[0].text.charCodeAt(1)).toBe(0xdf1f);
    });

    it('handles text with emoji where span covers ASCII and emoji', () => {
      // "a🎉b" — a(1) 🎉(4) b(1) = 6 bytes
      // Span [0,5] covers "a🎉"
      const text = 'a\ud83c\udf89b';
      const spans: IntraSpan[] = [{ start: 0, len: 5 }];
      const result = splitIntraText(text, spans);
      expect(result.length).toBe(2);
      expect(result[0].highlight).toBe(true);  // "a🎉"
      expect(result[1].highlight).toBe(false); // "b"
      expect(result[1].text).toBe('b');
      // First segment: "a" (1 char) + emoji (2 chars) = 3 JS chars
      expect(result[0].text.length).toBe(3);
      expect(result[0].text[0]).toBe('a');
    });

    it('handles consecutive surrogate pairs (two emojis in sequence)', () => {
      // "🌟🎉" — 🌟(4) 🎉(4) = 8 bytes
      // Span [4,4] covers only the second emoji
      const text = '\ud83c\udf1f\ud83c\udf89';
      const spans: IntraSpan[] = [{ start: 4, len: 4 }];
      const result = splitIntraText(text, spans);
      expect(result.length).toBe(2);
      expect(result[0].highlight).toBe(false); // first emoji
      expect(result[1].highlight).toBe(true);  // second emoji
      // Each segment should be a complete surrogate pair (2 JS chars each)
      expect(result[0].text.length).toBe(2);
      expect(result[1].text.length).toBe(2);
    });
  });

  describe('span ordering and edge cases', () => {
    it('handles spans that cover the entire text exactly', () => {
      const spans: IntraSpan[] = [
        { start: 0, len: 3 },
        { start: 3, len: 3 },
      ];
      const result = splitIntraText('foobar', spans);
      expect(result).toEqual([
        { text: 'foo', highlight: true },
        { text: 'bar', highlight: true },
      ]);
    });

    it('returns unhighlighted text when spans are out of range', () => {
      const spans: IntraSpan[] = [{ start: 100, len: 5 }];
      const result = splitIntraText('short', spans);
      expect(result).toEqual([{ text: 'short', highlight: false }]);
    });
  });
});

// ─── NativeDiffRenderer component tests ─────────────────────────────────

/** Create a minimal empty DiffResult */
function emptyDiff(): DiffResult {
  return { files: [], stats: { additions: 0, deletions: 0, unchanged: 0 }, warnings: [], truncated: false };
}

/** Create a DiffResult with one file that has no hunks (identical files) */
function identicalDiff(): DiffResult {
  return {
    files: [{ left: 'a.txt', right: 'b.txt', hunks: [] }],
    stats: { additions: 0, deletions: 0, unchanged: 0 },
    warnings: [],
    truncated: false,
  };
}

/** Create a DiffResult with context, additions, and deletions (no intra spans) */
function simpleDiff(): DiffResult {
  return {
    files: [
      {
        left: 'old.txt',
        right: 'new.txt',
        hunks: [
          {
            old_range: { start: 1, len: 5 },
            new_range: { start: 1, len: 5 },
            lines: [
              { kind: 'context', old_no: 1, new_no: 1, text: 'line 1', intra_spans: [] },
              { kind: 'context', old_no: 2, new_no: 2, text: 'line 2', intra_spans: [] },
              { kind: 'deletion', old_no: 3, new_no: null, text: 'old line 3', intra_spans: [] },
              { kind: 'addition', old_no: null, new_no: 3, text: 'new line 3', intra_spans: [] },
              { kind: 'context', old_no: 4, new_no: 4, text: 'line 4', intra_spans: [] },
            ],
            foldable_context: false,
          },
        ],
      },
    ],
    stats: { additions: 1, deletions: 1, unchanged: 3 },
    warnings: [],
    truncated: false,
  };
}

/** Create a DiffResult with intraline spans on the diff lines */
function diffWithIntraSpans(): DiffResult {
  return {
    files: [
      {
        left: 'old.txt',
        right: 'new.txt',
        hunks: [
          {
            old_range: { start: 1, len: 3 },
            new_range: { start: 1, len: 3 },
            lines: [
              { kind: 'deletion', old_no: 1, new_no: null, text: 'caf\u00e9', intra_spans: [{ start: 3, len: 2 }] },
              { kind: 'addition', old_no: null, new_no: 1, text: 'cafe', intra_spans: [{ start: 3, len: 1 }] },
              { kind: 'context', old_no: 2, new_no: 2, text: 'common', intra_spans: [] },
            ],
            foldable_context: false,
          },
        ],
      },
    ],
    stats: { additions: 1, deletions: 1, unchanged: 1 },
    warnings: [],
    truncated: false,
  };
}

/**
 * Render NativeDiffRenderer wrapped in SettingsProvider.
 * Optionally pre-set localStorage with settings overrides (e.g. { diffStyle: 'unified' }).
 */
function renderDiff(diff: DiffResult, storageOverride?: Record<string, unknown>) {
  // Clear any previous storage state
  localStorage.clear();
  if (storageOverride) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(storageOverride));
  }
  return render(
    <SettingsProvider>
      <NativeDiffRenderer diff={diff} />
    </SettingsProvider>,
  );
}

describe('NativeDiffRenderer', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  describe('empty / identical', () => {
    it('shows "No changes" message when diff has no files', () => {
      renderDiff(emptyDiff());
      expect(screen.getByText('No changes')).toBeInTheDocument();
    });

    it('shows "Files are identical" message when file has no hunks', () => {
      renderDiff(identicalDiff());
      expect(screen.getByText('Files are identical')).toBeInTheDocument();
    });
  });

  describe('unified view', () => {
    it('renders context, deletion, and addition lines', () => {
      renderDiff(simpleDiff(), { diffStyle: 'unified' });

      expect(screen.getByText('line 1')).toBeInTheDocument();
      expect(screen.getByText('line 2')).toBeInTheDocument();
      expect(screen.getByText('old line 3')).toBeInTheDocument();
      expect(screen.getByText('new line 3')).toBeInTheDocument();
      expect(screen.getByText('line 4')).toBeInTheDocument();
    });

    it('renders file header with left → right path', () => {
      renderDiff(simpleDiff(), { diffStyle: 'unified' });
      // The unified file header renders "old.txt → new.txt"
      expect(screen.getByText(/old\.txt/)).toBeInTheDocument();
      expect(screen.getByText(/new\.txt/)).toBeInTheDocument();
    });

    it('renders hunk header with @@ range notation', () => {
      renderDiff(simpleDiff(), { diffStyle: 'unified' });
      expect(screen.getByText(/@@ -1,5 \+1,5 @@/)).toBeInTheDocument();
    });

    it('renders markers for additions (+) and deletions (\u2212) in unified view', () => {
      renderDiff(simpleDiff(), { diffStyle: 'unified' });
      expect(screen.getByText('\u2212')).toBeInTheDocument();
      expect(screen.getByText('+')).toBeInTheDocument();
    });
  });

  describe('split view', () => {
    it('renders both left and right file headers', () => {
      renderDiff(simpleDiff());
      // Split view shows each file name in its own .file-header div
      const headers = document.querySelectorAll('.split-file-header .file-header');
      expect(headers.length).toBe(2);
      expect(headers[0].textContent).toBe('old.txt');
      expect(headers[1].textContent).toBe('new.txt');
    });

    it('renders context, deletion, and addition lines in split view', () => {
      renderDiff(simpleDiff());

      // In split view, context lines appear in both columns, so expect multiples
      expect(screen.getAllByText('line 1').length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText('line 2').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('old line 3')).toBeInTheDocument();
      expect(screen.getByText('new line 3')).toBeInTheDocument();
      expect(screen.getAllByText('line 4').length).toBeGreaterThanOrEqual(1);
    });

    it('renders line numbers in split view', () => {
      renderDiff(simpleDiff());
      // Each side has its own line numbers
      const lineNumbers = screen.getAllByText('1');
      expect(lineNumbers.length).toBeGreaterThan(0);
    });
  });

  describe('intraline span rendering', () => {
    it('renders intra-highlight spans for changed characters', () => {
      renderDiff(diffWithIntraSpans());
      const highlights = document.querySelectorAll('.intra-highlight');
      expect(highlights.length).toBe(2); // one for é, one for e
    });

    it('renders unicode intra-highlight for deletion line (caf\u00e9 → cafe)', () => {
      renderDiff(diffWithIntraSpans());

      // The accented é in "café" should be highlighted
      expect(screen.getByText('\u00e9')).toBeInTheDocument();
      // "caf" appears in both lines; use getAllByText
      const cafElements = screen.getAllByText('caf');
      expect(cafElements.length).toBeGreaterThanOrEqual(2);
    });

    it('renders intra-highlight for addition line changed character', () => {
      renderDiff(diffWithIntraSpans());

      // The "e" (from "cafe") that replaced "é" should be highlighted
      const highlights = document.querySelectorAll('.intra-highlight');
      const highlightTexts = Array.from(highlights).map((el) => el.textContent);
      expect(highlightTexts).toContain('\u00e9');
      expect(highlightTexts).toContain('e');
    });

    it('does not render intra-highlight on context lines', () => {
      renderDiff(diffWithIntraSpans());
      // The context line "common" should not have any highlight children
      // Find the context line element
      const contextLines = document.querySelectorAll('.diff-line.context, .diff-line-split.context');
      let hasContextHighlight = false;
      contextLines.forEach((line) => {
        if (line.querySelector('.intra-highlight')) {
          hasContextHighlight = true;
        }
      });
      expect(hasContextHighlight).toBe(false);
    });
  });

  describe('foldable context', () => {
    it('renders hunk separator between hunks with foldable context', () => {
      const diff: DiffResult = {
        files: [
          {
            left: 'a.txt',
            right: 'b.txt',
            hunks: [
              {
                old_range: { start: 1, len: 2 },
                new_range: { start: 1, len: 2 },
                lines: [
                  { kind: 'deletion', old_no: 1, new_no: null, text: 'del1', intra_spans: [] },
                  { kind: 'addition', old_no: null, new_no: 1, text: 'add1', intra_spans: [] },
                ],
                foldable_context: false,
              },
              {
                old_range: { start: 10, len: 2 },
                new_range: { start: 10, len: 2 },
                lines: [
                  { kind: 'deletion', old_no: 10, new_no: null, text: 'del10', intra_spans: [] },
                  { kind: 'addition', old_no: null, new_no: 10, text: 'add10', intra_spans: [] },
                ],
                foldable_context: true,
              },
            ],
          },
        ],
        stats: { additions: 2, deletions: 2, unchanged: 0 },
        warnings: [],
        truncated: false,
      };

      renderDiff(diff);
      expect(screen.getByText('\u00b7\u00b7\u00b7')).toBeInTheDocument();
    });
  });
});
