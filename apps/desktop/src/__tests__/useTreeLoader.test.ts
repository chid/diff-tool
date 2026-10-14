import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useTreeLoader } from '../useTreeLoader';
import { sampleTree } from '../sampleData';
import * as tauri from '../tauri';
import * as tauriDetect from '../tauriDetect';

describe('useTreeLoader', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('returns sampleTree when not in Tauri mode', () => {
    vi.spyOn(tauriDetect, 'isTauri', 'get').mockReturnValue(false);

    const { result } = renderHook(() => useTreeLoader());
    expect(result.current.entries).toEqual(sampleTree);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('loads single directory entries via scanTree in Tauri mode', async () => {
    vi.spyOn(tauriDetect, 'isTauri', 'get').mockReturnValue(true);
    const mockEntries = [
      { path: 'file1.txt', status: 'unchanged' as const, children: [] },
    ];
    vi.spyOn(tauri, 'scanTree').mockResolvedValue(mockEntries);

    const { result } = renderHook(() => useTreeLoader('/mock/root'));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
      expect(result.current.entries).toEqual(mockEntries);
      expect(result.current.error).toBeNull();
    });
  });

  it('loads two-folder comparison entries via compareFolders in Tauri mode', async () => {
    vi.spyOn(tauriDetect, 'isTauri', 'get').mockReturnValue(true);
    const mockDiff = {
      entries: [
        { path: 'added.txt', status: 'added' as const, children: [] },
        { path: 'modified.txt', status: 'modified' as const, children: [] },
      ],
    };
    vi.spyOn(tauri, 'compareFolders').mockResolvedValue(mockDiff);

    const { result } = renderHook(() =>
      useTreeLoader(undefined, '/mock/left', '/mock/right'),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
      expect(result.current.entries).toEqual(mockDiff.entries);
      expect(result.current.error).toBeNull();
    });
  });

  it('sets error state when compareFolders rejects in Tauri mode', async () => {
    vi.spyOn(tauriDetect, 'isTauri', 'get').mockReturnValue(true);
    vi.spyOn(tauri, 'compareFolders').mockRejectedValue(new Error('Folder not found'));

    const { result } = renderHook(() =>
      useTreeLoader(undefined, '/invalid/left', '/invalid/right'),
    );

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
      expect(result.current.entries).toEqual([]);
      expect(result.current.error).toBe('Folder not found');
    });
  });
});
