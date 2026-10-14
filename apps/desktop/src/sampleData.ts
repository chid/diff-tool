import type { DirEntryDiff } from './types';
import type { FileContents } from '@pierre/diffs';

export const sampleTree: DirEntryDiff[] = [
  { path: 'src/App.tsx', status: 'modified', size: 9200, children: [] },
  { path: 'src/components/DiffPane.tsx', status: 'added', size: 4700, children: [] },
  { path: 'src/components/FileTreePane.tsx', status: 'added', size: 3200, children: [] },
  { path: 'crates/diff-core/src/lib.rs', status: 'modified', size: 14800, children: [] },
  { path: 'docs/agent-work-packages.md', status: 'added', size: 2200, children: [] },
];

/** Full file contents for the old version of src/App.tsx. */
export const sampleOldFile: FileContents = {
  name: 'src/App.tsx',
  contents: [
    'import React from "react";',
    '',
    'export function App() {',
    '  return (',
    '    <main className="app-shell">',
    '      <h1>Diff Tool Prototype</h1>',
    '      <p>Select a file to compare.</p>',
    '    </main>',
    '  );',
    '}',
  ].join('\n'),
};

/** Full file contents for the new version of src/App.tsx. */
export const sampleNewFile: FileContents = {
  name: 'src/App.tsx',
  contents: [
    'import React, { useState } from "react";',
    'import { DiffPane } from "./components/DiffPane";',
    'import { FileTreePane } from "./components/FileTreePane";',
    '',
    'export default function App() {',
    '  const [style, setStyle] = useState<"unified" | "split">("split");',
    '',
    '  return (',
    '    <main className="app-shell">',
    '      <FileTreePane />',
    '      <section className="workspace">',
    '        <DiffPane mode={style} onToggle={() => setStyle(s => s === "split" ? "unified" : "split")} />',
    '      </section>',
    '    </main>',
    '  );',
    '}',
  ].join('\n'),
};

export const sampleStats = { additions: 8, deletions: 3, unchanged: 5 };

export interface SampleFileEntry {
  oldFile: FileContents;
  newFile: FileContents;
}

export const sampleFileMap: Record<string, SampleFileEntry> = {
  'src/App.tsx': {
    oldFile: sampleOldFile,
    newFile: sampleNewFile,
  },
  'src/components/DiffPane.tsx': {
    oldFile: { name: 'src/components/DiffPane.tsx', contents: '' },
    newFile: {
      name: 'src/components/DiffPane.tsx',
      contents: [
        'import React from "react";',
        'import { FileDiff } from "@pierre/diffs/react";',
        '',
        'interface DiffPaneProps {',
        '  mode: "unified" | "split";',
        '  onToggle: () => void;',
        '}',
        '',
        'export function DiffPane({ mode, onToggle }: DiffPaneProps) {',
        '  return (',
        '    <div className="diff-pane">',
        '      <header className="pane-header">',
        '        <button onClick={onToggle}>Style: {mode}</button>',
        '      </header>',
        '      <FileDiff options={{ diffStyle: mode }} />',
        '    </div>',
        '  );',
        '}',
      ].join('\n'),
    },
  },
  'src/components/FileTreePane.tsx': {
    oldFile: { name: 'src/components/FileTreePane.tsx', contents: '' },
    newFile: {
      name: 'src/components/FileTreePane.tsx',
      contents: [
        'import React from "react";',
        'import { FileTree } from "@pierre/trees/react";',
        '',
        'export function FileTreePane() {',
        '  return (',
        '    <aside className="tree-pane">',
        '      <h3>Changed Files (5)</h3>',
        '      <FileTree />',
        '    </aside>',
        '  );',
        '}',
      ].join('\n'),
    },
  },
  'crates/diff-core/src/lib.rs': {
    oldFile: {
      name: 'crates/diff-core/src/lib.rs',
      contents: [
        'pub enum DiffAlgorithm {',
        '    Myers,',
        '}',
        '',
        'pub fn run_diff(left: &str, right: &str) -> DiffResult {',
        '    imara_diff::diff(left, right)',
        '}',
      ].join('\n'),
    },
    newFile: {
      name: 'crates/diff-core/src/lib.rs',
      contents: [
        'pub enum DiffAlgorithm {',
        '    Myers,',
        '    Histogram,',
        '    Syntax,',
        '}',
        '',
        'pub fn run_diff(left: &str, right: &str, algo: DiffAlgorithm) -> DiffResult {',
        '    match algo {',
        '        DiffAlgorithm::Myers => imara_diff::myers(left, right),',
        '        DiffAlgorithm::Histogram => imara_diff::histogram(left, right),',
        '        DiffAlgorithm::Syntax => parser::tree_sitter_diff(left, right),',
        '    }',
        '}',
      ].join('\n'),
    },
  },
  'docs/agent-work-packages.md': {
    oldFile: { name: 'docs/agent-work-packages.md', contents: '' },
    newFile: {
      name: 'docs/agent-work-packages.md',
      contents: [
        '# Work Packages',
        '',
        '- Phase 1: High-performance Rust core with Myers & Histogram algorithms',
        '- Phase 2: AST syntax-aware diffing with Tree-sitter',
        '- Phase 3: Desktop UI with Pierre diff rendering and tree navigation',
        '- Phase 4: CLI tooling (`dt diff`, `dt dir`)',
        '- Phase 5: MCP server for AI coding agents',
      ].join('\n'),
    },
  },
};

