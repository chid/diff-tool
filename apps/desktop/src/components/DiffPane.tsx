import { FileDiff } from '@pierre/diffs/react';
import type { FileDiffMetadata, FileDiffOptions } from '@pierre/diffs';
import { Loader2, AlertTriangle, RefreshCw, FileX } from 'lucide-react';
import { useSettings } from './SettingsContext';
import { settingsToFileDiffOptions } from '../settingsTypes';
import { NativeDiffRenderer } from './NativeDiffRenderer';
import type { LoadState, DiffResult, DiffProgress } from '../types';

interface DiffPaneProps {
  fileDiff: FileDiffMetadata | null;
  nativeDiff: DiffResult | null;
  loadState: LoadState;
  onRetry: () => void;
  diffProgress?: DiffProgress | null;
}

const PROGRESS_LABELS: Record<string, string> = {
  reading: 'Reading files…',
  normalizing: 'Normalizing…',
  computing_diff: 'Computing diff…',
  computing_intraline_spans: 'Computing intraline spans…',
  building_hunks: 'Building hunks…',
};

const PROGRESS_PCT: Record<string, number> = {
  reading: 10,
  normalizing: 25,
  computing_diff: 55,
  computing_intraline_spans: 75,
  building_hunks: 95,
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

export function DiffPane({ fileDiff, nativeDiff, loadState, onRetry, diffProgress }: DiffPaneProps) {
  const { settings } = useSettings();

  if (loadState.phase === 'loading') {
    const stage = diffProgress?.stage;
    const label = stage ? PROGRESS_LABELS[stage] ?? stage : 'Reading files…';
    const detail = stage === 'reading' && diffProgress?.path
      ? diffProgress.path.split('/').pop() ?? diffProgress.path
      : undefined;
    const fileBytes = stage === 'reading' ? diffProgress?.bytes : undefined;

    return (
      <article className="diff-pane diff-pane-center" aria-label="Loading diff">
        <Loader2 className="diff-loader-icon" size={32} />
        <p className="diff-status-text">{label}</p>
        {detail && <p className="diff-status-detail">{detail}</p>}
        <div className="diff-progress-track" role="progressbar" aria-valuenow={stage ? PROGRESS_PCT[stage] : 0} aria-valuemin={0} aria-valuemax={100}>
          <div
            className="diff-progress-fill"
            style={{ width: `${stage ? PROGRESS_PCT[stage] : 5}%` }}
          />
        </div>
        {fileBytes !== undefined && fileBytes > 0 && (
          <p className="diff-status-hint">{formatBytes(fileBytes)}</p>
        )}
      </article>
    );
  }

  if (loadState.phase === 'error') {
    return (
      <article className="diff-pane diff-pane-center" aria-label="Diff load error">
        <div className="diff-error-card">
          <AlertTriangle className="diff-error-icon" size={28} />
          <div>
            <h3 className="diff-status-title">Failed to load diff</h3>
            <p className="diff-status-detail">{loadState.message}</p>
          </div>
          <button className="diff-retry-btn" onClick={onRetry}>
            <RefreshCw size={14} />
            Retry
          </button>
        </div>
      </article>
    );
  }

  if (!fileDiff) {
    return (
      <article className="diff-pane diff-pane-center" aria-label="No file selected">
        <FileX className="diff-empty-icon" size={36} />
        <p className="diff-status-text">Select a file to view changes</p>
        <p className="diff-status-hint">
          Open two files or a Git working tree from the sidebar
        </p>
      </article>
    );
  }

  return (
    <>
      {nativeDiff ? (
        <NativeDiffRenderer diff={nativeDiff} />
      ) : (
        <article className="diff-pane" aria-label="Text diff">
          <FileDiff
            fileDiff={fileDiff!}
            disableWorkerPool={settings.disableWorkerPool}
            options={settingsToFileDiffOptions(settings) as FileDiffOptions<undefined>}
            style={{ minHeight: 0, flex: '1 1 auto' }}
          />
        </article>
      )}
    </>
  );
}
