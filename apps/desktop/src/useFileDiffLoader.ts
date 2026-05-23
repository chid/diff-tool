import { useState, useEffect, useCallback } from 'react';
import { parseDiffFromFile } from '@pierre/diffs';
import type { FileContents } from '@pierre/diffs';
import type { LoadState } from './types';
import { isTauri } from './tauriDetect';
import { readFileText } from './tauri';
import { sampleOldFile, sampleNewFile } from './sampleData';

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
 * Re-triggers when `retryKey` changes.
 */
export function useFileDiffLoader(): {
  state: LoadState;
  retry: () => void;
} {
  const [retryKey, setRetryKey] = useState(0);
  const [state, setState] = useState<LoadState>({ phase: 'loading' });

  useEffect(() => {
    let cancelled = false;

    async function load() {
      if (!isTauri) {
        // Vite dev mode — use hardcoded mock data
        const fileDiff = parseDiffFromFile(sampleOldFile, sampleNewFile);
        if (!cancelled) setState({ phase: 'loaded', fileDiff });
        return;
      }

      setState({ phase: 'loading' });

      try {
        const [leftText, rightText] = await Promise.all([
          readFileText(DEMO_LEFT),
          readFileText(DEMO_RIGHT),
        ]);
        if (cancelled) return;

        const leftFile: FileContents = { name: DEMO_LEFT, contents: leftText };
        const rightFile: FileContents = { name: DEMO_RIGHT, contents: rightText };
        const fileDiff = parseDiffFromFile(leftFile, rightFile);
        setState({ phase: 'loaded', fileDiff });
      } catch (err) {
        if (!cancelled) {
          setState({
            phase: 'error',
            message: err instanceof Error ? err.message : String(err),
          });
        }
      }
    }

    load();
    return () => { cancelled = true; };
  }, [retryKey]);

  const retry = useCallback(() => setRetryKey((k) => k + 1), []);

  return { state, retry };
}
