import { useState, useEffect, useCallback } from 'react';
import { parseDiffFromFile } from '@pierre/diffs';
import type { FileContents, FileDiffMetadata } from '@pierre/diffs';
import type { ChangeKind, LoadState, DiffResult, DiffProgress, LineChange } from './types';
import { isTauri } from './tauriDetect';
import { readFileText, compareFiles, type CompareFilesOptions } from './tauri';
import { useSettings } from './components/SettingsContext';
import { sampleOldFile, sampleNewFile, sampleStats } from './sampleData';
import { listen } from '@tauri-apps/api/event';

function lineChangesFromText(text: string, kind: Exclude<ChangeKind, 'context'>): LineChange[] {
  const lines = text.length === 0 ? [] : text.replace(/\r\n?/g, '\n').split('\n');
  if (lines[lines.length - 1] === '') lines.pop();

  return lines.map((line, index) => ({
    kind,
    old_no: kind === 'deletion' ? index + 1 : null,
    new_no: kind === 'addition' ? index + 1 : null,
    text: line,
    intra_spans: [],
  }));
}

export function buildSingleFilePreviewDiff(path: string, text: string, side: 'left' | 'right'): DiffResult {
  const kind: Exclude<ChangeKind, 'context'> = side === 'left' ? 'deletion' : 'addition';
  const lines = lineChangesFromText(text, kind);
  const additions = kind === 'addition' ? lines.length : 0;
  const deletions = kind === 'deletion' ? lines.length : 0;
  const left = side === 'left' ? path : '(empty)';
  const right = side === 'right' ? path : '(empty)';

  return {
    files: lines.length === 0
      ? []
      : [{
          left,
          right,
          hunks: [{
            old_range: { start: 1, len: deletions },
            new_range: { start: 1, len: additions },
            lines,
            foldable_context: false,
          }],
        }],
    stats: { additions, deletions, unchanged: 0 },
    warnings: [],
    truncated: false,
  };
}

/**
 * Async hook that loads file contents and produces a {@link LoadState}.
 *
 * In Tauri mode:
 * - Both `leftPath` and `rightPath` provided: diffs them with the Rust
 *   compare_files command and renders the returned DiffResult directly.
 * - Only `rightPath` provided: loads the file as all-additions (single-file preview).
 * - Neither provided: stays idle until the user opens files or a folder.
 *
 * In Vite dev mode (no Tauri), uses hardcoded sample data regardless of paths.
 *
 * Re-triggers when `leftPath`, `rightPath`, or `retryKey` changes.
 */
export function useFileDiffLoader(
  leftPath?: string,
  rightPath?: string,
): {
  state: LoadState;
  stats: { additions: number; deletions: number; unchanged: number };
  nativeDiff: DiffResult | null;
  retry: () => void;
  diffProgress: DiffProgress | null;
} {
  const { settings } = useSettings();
  const [retryKey, setRetryKey] = useState(0);
  const [state, setState] = useState<LoadState>(isTauri ? { phase: 'idle' } : { phase: 'loading' });
  const [stats, setStats] = useState({ additions: 0, deletions: 0, unchanged: 0 });
  const [nativeDiff, setNativeDiff] = useState<DiffResult | null>(null);
  const [diffProgress, setDiffProgress] = useState<DiffProgress | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (!isTauri) {
        // Vite dev mode — use hardcoded mock data
        const fileDiff = parseDiffFromFile(sampleOldFile, sampleNewFile);
        if (!cancelled) {
          setState({ phase: 'loaded', fileDiff });
          setStats(sampleStats);
          setNativeDiff(null);
          setDiffProgress(null);
        }
        return;
      }

      if (!leftPath && !rightPath) {
        if (!cancelled) {
          setState({ phase: 'idle' });
          setStats({ additions: 0, deletions: 0, unchanged: 0 });
          setNativeDiff(null);
          setDiffProgress(null);
        }
        return;
      }

      setState({ phase: 'loading' });
      setDiffProgress(null);
      setNativeDiff(null);

      try {
        if (leftPath && rightPath) {
          const compareOpts: CompareFilesOptions = {
            algorithm: settings.diffAlgorithm,
            deadlineMs: settings.defaultDeadline > 0 ? settings.defaultDeadline : undefined,
            progress: true,
          };
          const unlisten = await listen<DiffProgress>('diff-progress', (event) => {
            if (!cancelled) setDiffProgress(event.payload);
          });
          try {
            const rustResult = await compareFiles(leftPath, rightPath, compareOpts);
            if (!cancelled) {
              setStats(rustResult.stats);
              setNativeDiff(rustResult);
              setState({ phase: 'loaded', fileDiff: null });
            }
          } finally {
            unlisten();
            if (!cancelled) setDiffProgress(null);
          }
        } else if (rightPath) {
          // Load the selected file's content as the "right" (new) side
          const text = await readFileText(rightPath);
          if (cancelled) return;
          const preview = buildSingleFilePreviewDiff(rightPath, text, 'right');
          setStats(preview.stats);
          setNativeDiff(preview);
          setState({ phase: 'loaded', fileDiff: null });
        } else if (leftPath) {
          // Only left path provided — load as all-deletions
          const text = await readFileText(leftPath);
          if (cancelled) return;
          const preview = buildSingleFilePreviewDiff(leftPath, text, 'left');
          setStats(preview.stats);
          setNativeDiff(preview);
          setState({ phase: 'loaded', fileDiff: null });
        }
      } catch (err) {
        if (!cancelled) {
          setState({
            phase: 'error',
            message: err instanceof Error ? err.message : String(err),
          });
          setDiffProgress(null);
        }
      }
    }

    load();
    return () => { cancelled = true; };
  }, [leftPath, rightPath, retryKey, settings.diffAlgorithm, settings.defaultDeadline]);

  const retry = useCallback(() => {
    setNativeDiff(null);
    setRetryKey((k) => k + 1);
  }, []);

  return { state, stats, nativeDiff, retry, diffProgress };
}
