// @pierre/trees web-components side-effect import – registers the custom element
import '@pierre/trees/web-components';
import { FileTree, useFileTree } from '@pierre/trees/react';
import type { GitStatusEntry } from '@pierre/trees';
import { useEffect } from 'react';
import type { DirEntryDiff } from '../types';

interface FileTreePaneProps {
  entries: DirEntryDiff[];
  activePath?: string;
  onSelect?: (path: string) => void;
}

/** Map our internal status labels to the GitStatus values @pierre/trees understands. */
function toGitStatus(status: DirEntryDiff['status']): GitStatusEntry['status'] | null {
  switch (status) {
    case 'added':     return 'added';
    case 'deleted':   return 'deleted';
    case 'modified':  return 'modified';
    case 'unchanged': return null;
    case 'directory': return null;
  }
}

export function FileTreePane({ entries, activePath, onSelect }: FileTreePaneProps) {
  const paths = entries.map((e) => e.path);

  const gitStatus: GitStatusEntry[] = entries
    .filter((e) => toGitStatus(e.status) !== null)
    .map((e) => ({
      path: e.path,
      status: toGitStatus(e.status) as GitStatusEntry['status'],
    }));

  const { model } = useFileTree({
    paths,
    gitStatus,
    initialSelectedPaths: activePath ? [activePath] : undefined,
    onSelectionChange: (selectedPaths) => {
      const first = selectedPaths[0];
      if (first && onSelect) onSelect(first);
    },
  });

  // Keep the model in sync whenever entries change after initial mount
  useEffect(() => {
    model.resetPaths(paths);
    model.setGitStatus(gitStatus);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries]);

  return (
    <section
      className="tree-panel"
      aria-label="Changed files"
      style={{ display: 'flex', flexDirection: 'column', minHeight: 0, flex: '1 1 0' }}
    >
      <div className="panel-heading">
        <span>Files</span>
        <span>{entries.length}</span>
      </div>
      <FileTree
        model={model}
        style={{ flex: '1 1 0', minHeight: 0 }}
      />
    </section>
  );
}
