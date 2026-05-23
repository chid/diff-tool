import { FileDiff } from '@pierre/diffs/react';
import type { FileDiffMetadata, FileDiffOptions } from '@pierre/diffs';
import { Loader2, AlertTriangle, RefreshCw, FileX } from 'lucide-react';
import { useSettings } from './SettingsContext';
import { settingsToFileDiffOptions } from '../settingsTypes';
import type { LoadState } from '../types';

interface DiffPaneProps {
  fileDiff: FileDiffMetadata | null;
  loadState: LoadState;
  onRetry: () => void;
}

export function DiffPane({ fileDiff, loadState, onRetry }: DiffPaneProps) {
  const { settings } = useSettings();

  if (loadState.phase === 'loading') {
    return (
      <article className="diff-pane diff-pane-center" aria-label="Loading diff">
        <Loader2 className="diff-loader-icon" size={32} />
        <p className="diff-status-text">Reading files…</p>
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
    <article className="diff-pane" aria-label="Text diff">
      <FileDiff
        fileDiff={fileDiff}
        disableWorkerPool={settings.disableWorkerPool}
        options={settingsToFileDiffOptions(settings) as FileDiffOptions<undefined>}
        style={{ minHeight: 0, flex: '1 1 auto' }}
      />
    </article>
  );
}
