import { useState, useEffect, useCallback } from 'react';
import { parseDiffFromFile } from '@pierre/diffs';
import type { FileContents, FileDiffMetadata } from '@pierre/diffs';
import type { LoadState, DiffResult, DiffProgress } from './types';
import { isTauri } from './tauriDetect';
import { readFileText, compareFiles, type CompareFilesOptions } from './tauri';
import { useSettings } from './components/SettingsContext';
import { sampleOldFile, sampleNewFile, sampleStats } from './sampleData';
import { listen } from '@tauri-apps/api/event';

/**
 * Current demo paths. These will be replaced by file-open dialogs in Phase 3.
 *
 * FIXME: These are relative to the Tauri app's working directory, which may
 * not be the project root. They work in `npm run dev` (Vite only) but will
 * likely fail under `cargo tauri dev` until absolute paths or a file-open
 * dialog is wired.
 */
const DEMO_LEFT = 'README.md';
const DEMO_RIGHT = 'TODO.md';

/**
 * Async hook that loads file contents and produces a {@link LoadState}.
 *
 * In Tauri mode:
 * - Both `leftPath` and `rightPath` provided: loads both files, diffs them,
 *   and fetches real stats from the Rust compare_files command.
 * - Only `rightPath` provided: loads the file as all-additions (single-file preview).
 * - Neither provided: loads the two hardcoded demo files with real Rust stats.
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
  const [state, setState] = useState<LoadState>({ phase: 'loading' });
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

      setState({ phase: 'loading' });
      setDiffProgress(null);

      try {
        let fileDiff: FileDiffMetadata;

        if (leftPath && rightPath) {
          // Two-file comparison from file dialogs
          const [leftText, rightText] = await Promise.all([
            readFileText(leftPath),
            readFileText(rightPath),
          ]);
          if (cancelled) return;
          const leftFile: FileContents = { name: leftPath, contents: leftText };
          const rightFile: FileContents = { name: rightPath, contents: rightText };
          fileDiff = parseDiffFromFile(leftFile, rightFile);

          // Fetch real stats from the Rust diff engine
          try {
            const compareOpts: CompareFilesOptions = {
              algorithm: settings.diffAlgorithm,
              deadlineMs: settings.defaultDeadline > 0 ? settings.defaultDeadline : undefined,
              progress: true,
            };
            // Listen for progress events *before* starting the comparison
            const unlisten = await listen<DiffProgress>('diff-progress', (event) => {
              if (!cancelled) setDiffProgress(event.payload);
            });
            try {
              const rustResult = await compareFiles(leftPath, rightPath, compareOpts);
              if (!cancelled) {
                setStats(rustResult.stats);
                setNativeDiff(rustResult);
              }
            } finally {
              unlisten();
              if (!cancelled) setDiffProgress(null);
            }
          } catch (statsErr) {
            console.warn('compare_files failed, using client-side stats:', statsErr);
          }
        } else if (rightPath) {
          // Load the selected file's content as the "right" (new) side
          const text = await readFileText(rightPath);
          if (cancelled) return;
          const rightFile: FileContents = { name: rightPath, contents: text };
          // Empty left side — everything renders as additions
          const leftFile: FileContents = { name: rightPath, contents: '' };
          fileDiff = parseDiffFromFile(leftFile, rightFile);

          // Single-file preview — no meaningful cross-file stats
          if (!cancelled) {
            setStats({ additions: 0, deletions: 0, unchanged: 0 });
            setNativeDiff(null);
          }
        } else if (leftPath) {
          // Only left path provided — load as all-deletions
          const text = await readFileText(leftPath);
          if (cancelled) return;
          const leftFile: FileContents = { name: leftPath, contents: text };
          const rightFile: FileContents = { name: leftPath, contents: '' };
          fileDiff = parseDiffFromFile(leftFile, rightFile);
          if (!cancelled) {
            setStats({ additions: 0, deletions: 0, unchanged: 0 });
            setNativeDiff(null);
          }
        } else {
          // Default: compare two real demo files
          const [leftText, rightText] = await Promise.all([
            readFileText(DEMO_LEFT),
            readFileText(DEMO_RIGHT),
          ]);
          if (cancelled) return;
          const leftFile: FileContents = { name: DEMO_LEFT, contents: leftText };
          const rightFile: FileContents = { name: DEMO_RIGHT, contents: rightText };
          fileDiff = parseDiffFromFile(leftFile, rightFile);

          // Fetch real stats from the Rust diff engine for the two demo files
          try {
            const compareOpts: CompareFilesOptions = {
              algorithm: settings.diffAlgorithm,
              deadlineMs: settings.defaultDeadline > 0 ? settings.defaultDeadline : undefined,
              progress: true,
            };
            const unlisten = await listen<DiffProgress>('diff-progress', (event) => {
              if (!cancelled) setDiffProgress(event.payload);
            });
            try {
              const rustResult = await compareFiles(DEMO_LEFT, DEMO_RIGHT, compareOpts);
              if (!cancelled) {
                setStats(rustResult.stats);
                setNativeDiff(rustResult);
              }
            } finally {
              unlisten();
              if (!cancelled) setDiffProgress(null);
            }
          } catch (statsErr) {
            console.warn('compare_files failed, using client-side stats:', statsErr);
          }
        }

        setState({ phase: 'loaded', fileDiff });
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
  }, [leftPath, rightPath, retryKey]);

  const retry = useCallback(() => {
    setNativeDiff(null);
    setRetryKey((k) => k + 1);
  }, []);

  return { state, stats, nativeDiff, retry, diffProgress };
}
