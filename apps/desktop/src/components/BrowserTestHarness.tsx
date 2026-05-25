/**
 * Browser visual test harness for NativeDiffRenderer.
 *
 * Renders all test fixture cases on one page so the browser-use agent
 * can visually verify rendering correctness. Access at /?test=diff.
 */
import { useState } from 'react';
import { NativeDiffRenderer } from './NativeDiffRenderer';
import type { DiffResult } from '../types';

// ─── Test fixtures ──────────────────────────────────────────────────────

/** Empty diff — no files loaded yet */
const EMPTY_DIFF: DiffResult = {
  files: [],
  stats: { additions: 0, deletions: 0, unchanged: 0 },
  warnings: [],
  truncated: false,
};

/** Identical files — no hunks */
const IDENTICAL_DIFF: DiffResult = {
  files: [{ left: 'src/App.tsx', right: 'src/App.tsx', hunks: [] }],
  stats: { additions: 0, deletions: 0, unchanged: 5 },
  warnings: [],
  truncated: false,
};

/** ASCII change — basic replacement: "old1" → "new1" with common suffix "1" */
const ASCII_DIFF: DiffResult = {
  files: [
    {
      left: 'data.txt',
      right: 'data.txt',
      hunks: [
        {
          old_range: { start: 1, len: 1 },
          new_range: { start: 1, len: 1 },
          lines: [
            { kind: 'deletion', old_no: 1, new_no: null, text: 'old1', intra_spans: [{ start: 0, len: 3 }] },
            { kind: 'addition', old_no: null, new_no: 1, text: 'new1', intra_spans: [{ start: 0, len: 3 }] },
          ],
          foldable_context: false,
        },
      ],
    },
  ],
  stats: { additions: 1, deletions: 1, unchanged: 0 },
  warnings: [],
  truncated: false,
};

/** Multi-hunk diff with context lines, additions, deletions, and foldable context */
const MULTI_HUNK_DIFF: DiffResult = {
  files: [
    {
      left: 'src/lib.rs',
      right: 'src/lib.rs',
      hunks: [
        {
          old_range: { start: 1, len: 3 },
          new_range: { start: 1, len: 3 },
          lines: [
            { kind: 'context', old_no: 1, new_no: 1, text: 'fn main() {', intra_spans: [] },
            { kind: 'deletion', old_no: 2, new_no: null, text: '    let x = 1;', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 2, text: '    let x = 42;', intra_spans: [{ start: 12, len: 2 }] },
            { kind: 'context', old_no: 3, new_no: 3, text: '    println!("{x}");', intra_spans: [] },
          ],
          foldable_context: false,
        },
        {
          old_range: { start: 10, len: 2 },
          new_range: { start: 10, len: 3 },
          lines: [
            { kind: 'deletion', old_no: 10, new_no: null, text: '    // TODO', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 10, text: '    // FIXME: refactor', intra_spans: [{ start: 7, len: 12 }] },
            { kind: 'addition', old_no: null, new_no: 11, text: '    bar();', intra_spans: [] },
            { kind: 'context', old_no: 11, new_no: 12, text: '}', intra_spans: [] },
          ],
          foldable_context: true,
        },
      ],
    },
  ],
  stats: { additions: 3, deletions: 2, unchanged: 2 },
  warnings: [],
  truncated: false,
};

/** Unicode — café → cafe (accented é replaced with plain e) */
const UNICODE_DIFF: DiffResult = {
  files: [
    {
      left: 'menu.txt',
      right: 'menu.txt',
      hunks: [
        {
          old_range: { start: 1, len: 2 },
          new_range: { start: 1, len: 2 },
          lines: [
            { kind: 'context', old_no: 1, new_no: 1, text: '=== Menu ===', intra_spans: [] },
            {
              kind: 'deletion',
              old_no: 2,
              new_no: null,
              text: 'café',
              intra_spans: [{ start: 3, len: 2 }],
            },
            {
              kind: 'addition',
              old_no: null,
              new_no: 2,
              text: 'cafe',
              intra_spans: [{ start: 3, len: 1 }],
            },
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

/** Emoji surrogate pair — "hi🎉bye" with 🎉 highlighted */
const EMOJI_DIFF: DiffResult = {
  files: [
    {
      left: 'greeting.txt',
      right: 'greeting.txt',
      hunks: [
        {
          old_range: { start: 1, len: 2 },
          new_range: { start: 1, len: 2 },
          lines: [
            {
              kind: 'deletion',
              old_no: 1,
              new_no: null,
              text: 'hi🎉old',
              intra_spans: [{ start: 2, len: 4 }],
            },
            {
              kind: 'addition',
              old_no: null,
              new_no: 1,
              text: 'hi✨new',
              intra_spans: [{ start: 2, len: 4 }],
            },
            { kind: 'context', old_no: 2, new_no: 2, text: 'world', intra_spans: [] },
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

/** Warning diff — simulates a binary/invalid UTF-8 warning */
const WARNING_DIFF: DiffResult = {
  files: [
    {
      left: 'image.png',
      right: 'image.png',
      hunks: [],
    },
  ],
  stats: { additions: 0, deletions: 0, unchanged: 0 },
  warnings: ['image.png: binary file detected, showing placeholder'],
  truncated: false,
};

// ─── Harness component ──────────────────────────────────────────────────

interface TestCase {
  label: string;
  description: string;
  diff: DiffResult;
  /** Override diffStyle for this specific case (defaults to global setting) */
  diffStyle?: 'unified' | 'split';
}

const TEST_CASES: TestCase[] = [
  { label: 'Empty', description: 'No files loaded — empty state', diff: EMPTY_DIFF },
  { label: 'Identical', description: 'Files are identical — no hunks', diff: IDENTICAL_DIFF },
  { label: 'ASCII spans', description: 'Basic ASCII intraline highlights', diff: ASCII_DIFF },
  { label: 'Unicode (café → cafe)', description: 'Multi-byte UTF-8 character span', diff: UNICODE_DIFF },
  { label: 'Emoji surrogate pairs', description: '4-byte UTF-8 emoji spans (🎉✨)', diff: EMOJI_DIFF },
  { label: 'Multi-hunk', description: 'Multiple hunks with foldable context separator', diff: MULTI_HUNK_DIFF },
  { label: 'Warnings', description: 'Warnings in diff result (binary file)', diff: WARNING_DIFF },
];

export function BrowserTestHarness() {
  const [currentIndex, setCurrentIndex] = useState(0);

  const goNext = () => setCurrentIndex((i) => (i + 1) % TEST_CASES.length);
  const goPrev = () => setCurrentIndex((i) => (i - 1 + TEST_CASES.length) % TEST_CASES.length);

  const current = TEST_CASES[currentIndex];

  return (
    <div className="browser-test-harness">
      <header className="harness-header">
        <h1>NativeDiffRenderer — Browser Visual Test</h1>
        <div className="harness-nav">
          <button onClick={goPrev} aria-label="Previous test case">◀ Prev</button>
          <span className="harness-counter">
            {currentIndex + 1} / {TEST_CASES.length}
          </span>
          <button onClick={goNext} aria-label="Next test case">Next ▶</button>
        </div>
      </header>

      <section className="harness-case-info" aria-label="Test case info">
        <h2>{current.label}</h2>
        <p>{current.description}</p>
      </section>

      <section className="harness-render-area" aria-label="Diff render area">
        <NativeDiffRenderer diff={current.diff} />
      </section>

      <footer className="harness-status" aria-label="Status bar">
        <span className="harness-badge" id="test-label">{current.label}</span>
        <span className="harness-badge" id="test-index">{currentIndex + 1}/{TEST_CASES.length}</span>
        <span className="harness-badge" id="stats-additions">+{current.diff.stats.additions}</span>
        <span className="harness-badge" id="stats-deletions">−{current.diff.stats.deletions}</span>
        {current.diff.warnings.length > 0 && (
          <span className="harness-badge harness-badge-warn" id="warning-count">
            ⚠ {current.diff.warnings.length}
          </span>
        )}
      </footer>
    </div>
  );
}
