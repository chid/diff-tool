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
