import type { DiffResult, DirEntryDiff } from './types';

export const sampleTree: DirEntryDiff[] = [
  { path: 'src/App.tsx', status: 'modified', size: 9200, children: [] },
  { path: 'src/components/DiffPane.tsx', status: 'added', size: 4700, children: [] },
  { path: 'src/components/FileTreePane.tsx', status: 'added', size: 3200, children: [] },
  { path: 'crates/diff-core/src/lib.rs', status: 'modified', size: 14800, children: [] },
  { path: 'docs/agent-work-packages.md', status: 'added', size: 2200, children: [] },
];

export const sampleDiff: DiffResult = {
  files: [
    {
      left: 'src/App.tsx',
      right: 'src/App.tsx',
      hunks: [
        {
          old_range: { start: 1, len: 7 },
          new_range: { start: 1, len: 8 },
          foldable_context: false,
          lines: [
            { kind: 'context', old_no: 1, new_no: 1, text: 'import React from "react";', intra_spans: [] },
            { kind: 'deletion', old_no: 2, new_no: null, text: 'export function App() {', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 2, text: 'export default function App() {', intra_spans: [] },
            { kind: 'context', old_no: 3, new_no: 3, text: '  return (', intra_spans: [] },
            { kind: 'deletion', old_no: 4, new_no: null, text: '    <main>Diff prototype</main>', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 4, text: '    <main className="workspace">', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 5, text: '      <DiffWorkbench />', intra_spans: [] },
            { kind: 'addition', old_no: null, new_no: 6, text: '    </main>', intra_spans: [] },
            { kind: 'context', old_no: 5, new_no: 7, text: '  );', intra_spans: [] },
            { kind: 'context', old_no: 6, new_no: 8, text: '}', intra_spans: [] },
          ],
        },
      ],
    },
  ],
  stats: { additions: 4, deletions: 2, unchanged: 4 },
  warnings: [],
  truncated: false,
};
