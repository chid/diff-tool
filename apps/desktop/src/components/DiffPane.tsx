import { FileDiff } from '@pierre/diffs/react';
import type { FileDiffMetadata, FileDiffOptions } from '@pierre/diffs';
import { useSettings } from './SettingsContext';
import { settingsToFileDiffOptions } from '../settingsTypes';

interface DiffPaneProps {
  fileDiff: FileDiffMetadata;
}

export function DiffPane({ fileDiff }: DiffPaneProps) {
  const { settings } = useSettings();

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
