import { useState, useEffect } from 'react';
import type { DirEntryDiff } from './types';
import { isTauri } from './tauriDetect';
import { scanTree } from './tauri';
import { sampleTree } from './sampleData';

/** FIXME: Relative path — replace with file-open dialog in Phase 3. */
const DEMO_ROOT = '.';

/**
 * Async hook that loads directory entries via Tauri IPC,
 * falling back to {@link sampleTree} in Vite dev mode.
 */
export function useTreeLoader(): DirEntryDiff[] {
  const [entries, setEntries] = useState<DirEntryDiff[]>(
    isTauri ? [] : sampleTree,
  );

  useEffect(() => {
    if (!isTauri) return;

    let cancelled = false;

    scanTree(DEMO_ROOT)
      .then((result) => {
        if (!cancelled) setEntries(result);
      })
      .catch((err) => {
        // Demo root may not be valid until Phase 3 wires a folder picker.
        console.warn('scan_tree failed:', err);
        if (!cancelled) setEntries([]);
      });

    return () => { cancelled = true; };
  }, []);

  return entries;
}
