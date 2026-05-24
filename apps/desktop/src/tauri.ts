import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { DiffResult, DirEntryDiff } from './types';

export async function compareFiles(left: string, right: string): Promise<DiffResult> {
  return invoke<DiffResult>('compare_files', { request: { left, right } });
}

export async function scanTree(root: string): Promise<DirEntryDiff[]> {
  return invoke<DirEntryDiff[]>('scan_tree', { root });
}

/** Read raw file contents from disk via Tauri IPC. */
export async function readFileText(
  path: string,
  maxBytes?: number
): Promise<string> {
  return invoke<string>('read_file_text', { path, maxBytes });
}

/** Open a native file picker dialog and return the selected path, or null if cancelled. */
export async function openFileDialog(): Promise<string | null> {
  const result = await open({
    multiple: false,
    title: 'Choose a file to compare',
  });
  if (result === null || Array.isArray(result)) return null;
  return result;
}
