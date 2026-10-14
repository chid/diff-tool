import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { DiffResult, DirEntryDiff, FolderDiff, OfficeDiffResult, AiCompareRequest, AiCompareResponse } from './types';

export interface CompareFilesOptions {
  algorithm?: 'myers' | 'histogram' | 'patience';
  deadlineMs?: number;
  /** Enable progress events via the Tauri event bus (`diff-progress`). */
  progress?: boolean;
}

export async function compareFiles(
  left: string,
  right: string,
  options?: CompareFilesOptions,
): Promise<DiffResult> {
  return invoke<DiffResult>('compare_files', {
    request: {
      left,
      right,
      algorithm: options?.algorithm,
      deadline_ms: options?.deadlineMs,
      progress: options?.progress ?? false,
    },
  });
}

export async function compareFolders(
  left: string,
  right: string,
): Promise<FolderDiff> {
  return invoke<FolderDiff>('compare_folders', { left, right });
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

/** Returns true if the given path exists and is a directory. */
export async function pathIsDir(path: string): Promise<boolean> {
  return invoke<boolean>('path_is_dir', { path });
}

/** Returns the command-line args the process was launched with (excluding argv[0]). */
export async function getLaunchArgs(): Promise<string[]> {
  return invoke<string[]>('get_launch_args');
}

/** Open a native file picker dialog and return the selected path, or null if cancelled. */
export async function openFileDialog(): Promise<string | null> {
  const result = await open({
    multiple: false,
    title: 'Choose a file to compare',
    filters: [{ name: 'All Files', extensions: ['*'] }],
  });
  if (result === null || Array.isArray(result)) return null;
  return result;
}

/** Open a native folder picker dialog and return the selected path, or null if cancelled. */
export async function openFolderDialog(): Promise<string | null> {
  const result = await open({
    multiple: false,
    directory: true,
    title: 'Choose a folder to scan',
  });
  if (result === null || Array.isArray(result)) return null;
  return result;
}

/** Compare two Office documents (.docx, .pptx, .xlsx) via the Tauri backend. */
export async function compareOffice(
  left: string,
  right: string,
  sheets?: string[],
): Promise<OfficeDiffResult> {
  return invoke<OfficeDiffResult>('compare_office', {
    request: {
      left,
      right,
      sheets: sheets ?? [],
    },
  });
}

/** Send a chat completion to an LLM provider via the Tauri backend. */
export async function compareAi(
  request: AiCompareRequest,
): Promise<AiCompareResponse> {
  return invoke<AiCompareResponse>('compare_ai', { request });
}
