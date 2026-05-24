// @pierre/trees web-components side-effect import – registers the custom element
import '@pierre/trees/web-components';
import { FileTree, useFileTree } from '@pierre/trees/react';
import type { GitStatusEntry, ContextMenuItem, ContextMenuOpenContext } from '@pierre/trees';
import { useEffect, useCallback } from 'react';
import type { DirEntryDiff } from '../types';

interface FileTreePaneProps {
  entries: DirEntryDiff[];
  activePath?: string;
  leftPath?: string;
  rightPath?: string;
  onSelect?: (path: string) => void;
  onSetLeft?: (path: string) => void;
  onSetRight?: (path: string) => void;
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

export function FileTreePane({ entries, activePath, leftPath, rightPath, onSelect, onSetLeft, onSetRight }: FileTreePaneProps) {
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
    composition: {
      contextMenu: {
        enabled: true,
        triggerMode: 'right-click',
      },
    },
  });

  // Keep the model in sync whenever entries change after initial mount
  useEffect(() => {
    model.resetPaths(paths);
    model.setGitStatus(gitStatus);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entries]);

  const renderContextMenu = useCallback(
    (item: ContextMenuItem, ctx: ContextMenuOpenContext) => {
      if (item.kind !== 'file') return null;

      const handleLeft = () => {
        ctx.close();
        onSetLeft?.(item.path);
      };

      const handleRight = () => {
        ctx.close();
        onSetRight?.(item.path);
      };

      const isLeft = leftPath === item.path;
      const isRight = rightPath === item.path;

      return (
        <div className="tree-context-menu" role="menu" aria-label="File actions">
          <button
            className={`tree-context-item ${isLeft ? 'active' : ''}`}
            role="menuitem"
            onClick={handleLeft}
          >
            <span className="tree-context-icon">◀</span>
            Set as left file
            {isLeft && <span className="tree-context-check">✓</span>}
          </button>
          <button
            className={`tree-context-item ${isRight ? 'active' : ''}`}
            role="menuitem"
            onClick={handleRight}
          >
            <span className="tree-context-icon">▶</span>
            Set as right file
            {isRight && <span className="tree-context-check">✓</span>}
          </button>
        </div>
      );
    },
    [leftPath, rightPath, onSetLeft, onSetRight],
  );

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
        renderContextMenu={renderContextMenu}
        style={{ flex: '1 1 0', minHeight: 0 }}
      />
    </section>
  );
}
