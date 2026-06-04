import { describe, expect, it } from 'vitest';
import { buildSingleFilePreviewDiff } from '../useFileDiffLoader';

describe('buildSingleFilePreviewDiff', () => {
  it('builds an all-additions native DiffResult for a right-side preview', () => {
    const diff = buildSingleFilePreviewDiff('/tmp/right.txt', 'alpha\nbeta\n', 'right');

    expect(diff.stats).toEqual({ additions: 2, deletions: 0, unchanged: 0 });
    expect(diff.files[0].left).toBe('(empty)');
    expect(diff.files[0].right).toBe('/tmp/right.txt');
    expect(diff.files[0].hunks[0].new_range).toEqual({ start: 1, len: 2 });
    expect(diff.files[0].hunks[0].lines.map((line) => line.kind)).toEqual([
      'addition',
      'addition',
    ]);
    expect(diff.files[0].hunks[0].lines.map((line) => line.new_no)).toEqual([1, 2]);
  });

  it('builds an all-deletions native DiffResult for a left-side preview', () => {
    const diff = buildSingleFilePreviewDiff('/tmp/left.txt', 'alpha\nbeta', 'left');

    expect(diff.stats).toEqual({ additions: 0, deletions: 2, unchanged: 0 });
    expect(diff.files[0].left).toBe('/tmp/left.txt');
    expect(diff.files[0].right).toBe('(empty)');
    expect(diff.files[0].hunks[0].old_range).toEqual({ start: 1, len: 2 });
    expect(diff.files[0].hunks[0].lines.map((line) => line.kind)).toEqual([
      'deletion',
      'deletion',
    ]);
    expect(diff.files[0].hunks[0].lines.map((line) => line.old_no)).toEqual([1, 2]);
  });

  it('normalizes CRLF and returns no file hunks for an empty preview', () => {
    const crlf = buildSingleFilePreviewDiff('/tmp/right.txt', 'alpha\r\nbeta\r\n', 'right');
    const empty = buildSingleFilePreviewDiff('/tmp/empty.txt', '', 'right');

    expect(crlf.files[0].hunks[0].lines.map((line) => line.text)).toEqual(['alpha', 'beta']);
    expect(empty.files).toEqual([]);
    expect(empty.stats).toEqual({ additions: 0, deletions: 0, unchanged: 0 });
  });
});
