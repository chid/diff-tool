import { useState, useEffect } from 'react';
import type { DirEntryDiff } from './types';
import { isTauri } from './tauriDetect';
import { scanTree } from './tauri';
import { sampleTree } from './sampleData';

/**
 * Async hook that loads directory entries via Tauri IPC,
 * falling back to {@link sampleTree} in Vite dev mode.
 *
 * When `rootPath` is provided, scans that directory.
 * When `rootPath` is undefined, uses sample data (dev) or empty (Tauri).
 */
export function useTreeLoader(rootPath?: string): DirEntryDiff[] {
  const [entries, setEntries] = useState<DirEntryDiff[]>(
    isTauri ? [] : sampleTree,
  );

  useEffect(() => {
    if (!isTauri || !rootPath) return;

    let cancelled = false;

    setEntries([]);
    scanTree(rootPath)
      .then((result) => {
        if (!cancelled) setEntries(result);
      })
      .catch((err) => {
        console.warn('scan_tree failed:', err);
        if (!cancelled) setEntries([]);
      });

    return () => { cancelled = true; };
  }, [rootPath]);

  return entries;
}
