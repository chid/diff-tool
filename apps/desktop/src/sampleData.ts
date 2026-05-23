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
    'export function App() {',
    '  return (',
    '    <main>Diff prototype</main>',
    '  );',
    '}',
  ].join('\n'),
};

/** Full file contents for the new version of src/App.tsx. */
export const sampleNewFile: FileContents = {
  name: 'src/App.tsx',
  contents: [
    'import React from "react";',
    'export default function App() {',
    '  return (',
    '    <main className="workspace">',
    '      <DiffWorkbench />',
    '    </main>',
    '  );',
    '}',
  ].join('\n'),
};

export const sampleStats = { additions: 4, deletions: 2, unchanged: 4 };
