import { GitBranch, Image, Layers3, SplitSquareHorizontal, TerminalSquare } from 'lucide-react';
import { DiffPane } from './components/DiffPane';
import { FileTreePane } from './components/FileTreePane';
import { sampleDiff, sampleTree } from './sampleData';

export function App() {
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
        <FileTreePane entries={sampleTree} activePath="src/App.tsx" />
      </aside>
      <section className="workspace" aria-label="Diff workspace">
        <header className="toolbar">
          <div>
            <h1>Working Tree</h1>
            <p>{sampleDiff.stats.additions} additions, {sampleDiff.stats.deletions} deletions</p>
          </div>
          <div className="toolbar-actions">
            <button>Unified</button>
            <button className="active">Split</button>
            <button>Whitespace</button>
          </div>
        </header>
        <DiffPane diff={sampleDiff} />
      </section>
    </main>
  );
}
