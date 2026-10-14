import type { FileDiffMetadata } from '@pierre/diffs';

/** Discriminated union for async file-diff loading state. */
export type LoadState =
  | { phase: 'idle' }
  | { phase: 'loading' }
  | { phase: 'loaded'; fileDiff?: FileDiffMetadata | null }
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

// ── Office document diff types ──────────────────────────────────────

export type OfficeFormat = 'word' | 'power-point' | 'excel';

export type ChangeStatus = 'added' | 'deleted' | 'modified' | 'unchanged' | 'reordered';

export type OfficeSectionKind =
  | 'word-paragraph'
  | 'word-heading'
  | 'word-table'
  | 'word-comment'
  | 'word-header'
  | 'word-footer'
  | 'power-point-slide'
  | 'power-point-notes'
  | 'excel-sheet'
  | 'excel-named-range';

export type OfficeAtomicChangeKind =
  | 'text-changed'
  | 'text-added'
  | 'text-deleted'
  | 'structure-changed'
  | 'formatting-changed'
  | 'image-changed'
  | 'formula-changed'
  | 'sheet-renamed'
  | 'slide-reordered';

export interface OfficeAtomicChange {
  path: string;
  kind: OfficeAtomicChangeKind;
  before: string | null;
  after: string | null;
  metadata: Record<string, unknown> | null;
}

export interface OfficeSectionDiff {
  id: string;
  label: string;
  kind: OfficeSectionKind;
  status: ChangeStatus;
  changes: OfficeAtomicChange[];
}

export interface OfficeDiffSummary {
  sections_total: number;
  sections_added: number;
  sections_deleted: number;
  sections_changed: number;
  sections_unchanged: number;
  total_atomic_changes: number;
}

export interface OfficeDiffResult {
  format: OfficeFormat;
  summary: OfficeDiffSummary;
  sections: OfficeSectionDiff[];
  warnings: string[];
  unsupported_features: string[];
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

export interface FolderDiff {
  entries: DirEntryDiff[];
}

// ── AI compare types ───────────────────────────────────────────────

export type AiProvider = 'openai' | 'anthropic' | 'custom';

export interface AiChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface AiCompareRequest {
  provider: AiProvider;
  api_key: string;
  model?: string;
  base_url?: string;
  max_tokens?: number;
  temperature?: number;
  messages: AiChatMessage[];
}

export interface AiCompareResponse {
  content: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_tokens?: number;
}
