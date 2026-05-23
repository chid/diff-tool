import { useState, useMemo } from 'react';
import { GitBranch, Image, Layers3, SplitSquareHorizontal, TerminalSquare } from 'lucide-react';
import { DiffPane } from './components/DiffPane';
import { FileTreePane } from './components/FileTreePane';
import { SettingsPanel } from './components/SettingsPanel';
import { SettingsProvider, useSettings } from './components/SettingsContext';
import { useFileDiffLoader } from './useFileDiffLoader';
import { useTreeLoader } from './useTreeLoader';

function AppInner() {
  const [activePath, setActivePath] = useState<string | undefined>('src/App.tsx');
  const { settings, updateSetting } = useSettings();

  const { state: loadState, retry } = useFileDiffLoader();

  const fileDiff = useMemo(
    () => (loadState.phase === 'loaded' ? loadState.fileDiff : null),
    [loadState],
  );

  // Real stats come from Rust compare_files in a later phase.
  // For now the toolbar subtitle is hidden until stats are wired.

  const treeEntries = useTreeLoader();

  const handleTreeSelect = (path: string) => {
    setActivePath(path);
  };

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <SplitSquareHorizontal size={20} />
          <span>Diff Tool</span>
        </div>
        <nav className="mode-list" aria-label="Comparison modes">
          <button className="active"><GitBranch size={16} /> Changes</button>
          <button><Layers3 size={16} /> Folders</button>
          <button><Image size={16} /> Images</button>
          <button><TerminalSquare size={16} /> CLI</button>
        </nav>
        <FileTreePane
          entries={treeEntries}
          activePath={activePath}
          onSelect={handleTreeSelect}
        />
      </aside>
      <section className="workspace" aria-label="Diff workspace">
        <header className="toolbar">
          <div>
            <h1>Working Tree</h1>
          </div>
          <div className="toolbar-actions">
            <button
              className={settings.diffStyle === 'unified' ? 'active' : ''}
              onClick={() => updateSetting('diffStyle', 'unified')}
            >
              Unified
            </button>
            <button
              className={settings.diffStyle === 'split' ? 'active' : ''}
              onClick={() => updateSetting('diffStyle', 'split')}
            >
              Split
            </button>
            <SettingsPanel />
          </div>
        </header>
        <DiffPane
          fileDiff={fileDiff}
          loadState={loadState}
          onRetry={retry}
        />
      </section>
    </main>
  );
}

export function App() {
  return (
    <SettingsProvider>
      <AppInner />
    </SettingsProvider>
  );
}
