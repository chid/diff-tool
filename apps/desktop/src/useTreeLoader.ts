import { useState, useEffect } from 'react';
import type { DirEntryDiff } from './types';
import { isTauri } from './tauriDetect';
import { scanTree, compareFolders } from './tauri';
import { sampleTree } from './sampleData';

export interface TreeLoaderResult {
  entries: DirEntryDiff[];
  loading: boolean;
  error: string | null;
}

/**
 * Async hook that loads directory entries via Tauri IPC,
 * falling back to {@link sampleTree} in Vite dev mode.
 *
 * - When both `leftFolderPath` and `rightFolderPath` are provided:
 *   runs two-folder comparison via `compare_folders`.
 * - When only `rootPath` is provided:
 *   scans that single directory via `scan_tree`.
 * - When none provided:
 *   uses sample data (dev) or empty (Tauri).
 */
export function useTreeLoader(
  rootPath?: string,
  leftFolderPath?: string,
  rightFolderPath?: string,
): TreeLoaderResult {
  const [entries, setEntries] = useState<DirEntryDiff[]>(
    isTauri ? [] : sampleTree,
  );
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri) {
      setEntries(sampleTree);
      setLoading(false);
      setError(null);
      return;
    }

    let cancelled = false;

    if (leftFolderPath && rightFolderPath) {
      setLoading(true);
      setError(null);
      setEntries([]);

      compareFolders(leftFolderPath, rightFolderPath)
        .then((result) => {
          if (!cancelled) {
            setEntries(result.entries);
            setLoading(false);
          }
        })
        .catch((err) => {
          console.warn('compare_folders failed:', err);
          if (!cancelled) {
            setError(err instanceof Error ? err.message : String(err));
            setEntries([]);
            setLoading(false);
          }
        });

      return () => {
        cancelled = true;
      };
    }

    if (rootPath) {
      setLoading(true);
      setError(null);
      setEntries([]);

      scanTree(rootPath)
        .then((result) => {
          if (!cancelled) {
            setEntries(result);
            setLoading(false);
          }
        })
        .catch((err) => {
          console.warn('scan_tree failed:', err);
          if (!cancelled) {
            setError(err instanceof Error ? err.message : String(err));
            setEntries([]);
            setLoading(false);
          }
        });

      return () => {
        cancelled = true;
      };
    }

    // Neither rootPath nor dual folders provided
    setEntries([]);
    setLoading(false);
    setError(null);
  }, [rootPath, leftFolderPath, rightFolderPath]);

  return { entries, loading, error };
}
