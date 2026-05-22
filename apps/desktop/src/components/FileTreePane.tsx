import type { DirEntryDiff } from '../types';

interface FileTreePaneProps {
  entries: DirEntryDiff[];
  activePath: string;
}

const statusLabels: Record<DirEntryDiff['status'], string> = {
  added: 'A',
  deleted: 'D',
  modified: 'M',
  unchanged: '',
  directory: '',
};

export function FileTreePane({ entries, activePath }: FileTreePaneProps) {
  return (
    <section className="tree-panel" aria-label="Changed files">
      <div className="panel-heading">
        <span>Files</span>
        <span>{entries.length}</span>
      </div>
      <div className="tree-search" role="search">
        <input type="search" placeholder="Filter paths" aria-label="Filter paths" />
      </div>
      <ul className="tree-list">
        {entries.map((entry) => (
          <li key={entry.path}>
            <button className={entry.path === activePath ? 'tree-row active' : 'tree-row'}>
              <span className={`status ${entry.status}`}>{statusLabels[entry.status]}</span>
              <span className="path" title={entry.path}>{entry.path}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
