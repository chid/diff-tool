import { invoke } from '@tauri-apps/api/core';
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
