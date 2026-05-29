import type { FileDiffMetadata } from '@pierre/diffs';

/** Discriminated union for async file-diff loading state. */
export type LoadState =
  | { phase: 'idle' }
  | { phase: 'loading' }
  | { phase: 'loaded'; fileDiff: FileDiffMetadata }
  | { phase: 'error'; message: string };

/** Progress event sent from the Rust backend during diff computation. */
export interface DiffProgress {
  stage: 'reading' | 'normalizing' | 'computing_diff' | 'computing_intraline_spans' | 'building_hunks';
  path?: string;
  bytes?: number;
}

export type ChangeKind = 'context' | 'addition' | 'deletion';

export interface IntraSpan {
  start: number;
  len: number;
}

export interface LineChange {
  kind: ChangeKind;
  old_no: number | null;
  new_no: number | null;
  text: string;
  intra_spans: IntraSpan[];
}

export interface Hunk {
  old_range: { start: number; len: number };
  new_range: { start: number; len: number };
  lines: LineChange[];
  foldable_context: boolean;
}

export interface FileDiff {
  left: string;
  right: string;
  hunks: Hunk[];
}

export interface DiffResult {
  files: FileDiff[];
  stats: { additions: number; deletions: number; unchanged: number };
  warnings: string[];
  truncated: boolean;
}

export type EntryStatus = 'added' | 'deleted' | 'modified' | 'unchanged' | 'directory';

export interface DirEntryDiff {
  path: string;
  status: EntryStatus;
  size?: number | null;
  mtime?: number | null;
  hash?: string | null;
  children: DirEntryDiff[];
}
