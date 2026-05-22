import { invoke } from '@tauri-apps/api/core';
import type { DiffResult, DirEntryDiff } from './types';

export async function compareFiles(left: string, right: string): Promise<DiffResult> {
  return invoke<DiffResult>('compare_files', { request: { left, right } });
}

export async function scanTree(root: string): Promise<DirEntryDiff[]> {
  return invoke<DirEntryDiff[]>('scan_tree', { root });
}
