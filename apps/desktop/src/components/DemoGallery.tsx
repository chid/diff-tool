import { useState } from 'react';
import {
  FileText,
  FileCode2,
  FileJson,
  Layers3,
  ChevronRight,
  Plus,
  ArrowRightLeft,
} from 'lucide-react';
import { NativeDiffRenderer } from './NativeDiffRenderer';
import type { DiffResult } from '../types';

// ── Pre-built example diffs ─────────────────────────────────────────────

/** "welcome" → "hello": a single-word replacement with intraline spans. */
const EXAMPLE_BASIC: DiffResult = {
  files: [{
    left: 'greeting.txt',
    right: 'greeting.txt',
    hunks: [{
      old_range: { start: 1, len: 3 },
      new_range: { start: 1, len: 3 },
      lines: [
        { kind: 'context', old_no: 1, new_no: 1, text: 'alpha', intra_spans: [] },
        {
          kind: 'deletion', old_no: 2, new_no: null, text: 'welcome',
          intra_spans: [{ start: 0, len: 7 }],
        },
        {
          kind: 'addition', old_no: null, new_no: 2, text: 'hello',
          intra_spans: [{ start: 0, len: 5 }],
        },
        { kind: 'context', old_no: 3, new_no: 3, text: 'gamma', intra_spans: [] },
      ],
      foldable_context: false,
    }],
  }],
  stats: { additions: 1, deletions: 1, unchanged: 2 },
  warnings: [],
  truncated: false,
};

/** A multi-hunk change: two modifications separated by context lines. */
const EXAMPLE_MULTI_HUNK: DiffResult = {
  files: [{
    left: 'config.toml',
    right: 'config.toml',
    hunks: [
      {
        old_range: { start: 1, len: 4 },
        new_range: { start: 1, len: 4 },
        lines: [
          { kind: 'context', old_no: 1, new_no: 1, text: '[package]', intra_spans: [] },
          {
            kind: 'deletion', old_no: 2, new_no: null, text: 'name = "old-name"',
            intra_spans: [{ start: 7, len: 8 }],
          },
          {
            kind: 'addition', old_no: null, new_no: 2, text: 'name = "diff-tool"',
            intra_spans: [{ start: 7, len: 9 }],
          },
          { kind: 'context', old_no: 3, new_no: 3, text: 'version = "0.1.0"', intra_spans: [] },
          { kind: 'context', old_no: 4, new_no: 4, text: '', intra_spans: [] },
        ],
        foldable_context: false,
      },
      {
        old_range: { start: 7, len: 4 },
        new_range: { start: 7, len: 4 },
        lines: [
          { kind: 'context', old_no: 7, new_no: 7, text: '[dependencies]', intra_spans: [] },
          {
            kind: 'deletion', old_no: 8, new_no: null, text: 'serde = "1.0"',
            intra_spans: [],
          },
          {
            kind: 'addition', old_no: null, new_no: 8, text: 'serde = { version = "1.0", features = ["derive"] }',
            intra_spans: [{ start: 8, len: 40 }],
          },
          { kind: 'context', old_no: 9, new_no: 9, text: 'thiserror = "1.0"', intra_spans: [] },
          { kind: 'context', old_no: 10, new_no: 10, text: '', intra_spans: [] },
        ],
        foldable_context: true,
      },
    ],
  }],
  stats: { additions: 2, deletions: 2, unchanged: 5 },
  warnings: [],
  truncated: false,
};

/** A pure-addition file: everything is new. */
const EXAMPLE_ADDED: DiffResult = {
  files: [{
    left: 'new-feature.rs',
    right: 'new-feature.rs',
    hunks: [{
      old_range: { start: 0, len: 0 },
      new_range: { start: 1, len: 6 },
      lines: [
        { kind: 'addition', old_no: null, new_no: 1, text: '/// An async function that fetches data.', intra_spans: [] },
        { kind: 'addition', old_no: null, new_no: 2, text: 'pub async fn fetch_data(url: &str) -> Result<Data, Error> {', intra_spans: [] },
        { kind: 'addition', old_no: null, new_no: 3, text: '    let response = reqwest::get(url).await?;', intra_spans: [] },
        { kind: 'addition', old_no: null, new_no: 4, text: '    let data: Data = response.json().await?;', intra_spans: [] },
        { kind: 'addition', old_no: null, new_no: 5, text: '    Ok(data)', intra_spans: [] },
        { kind: 'addition', old_no: null, new_no: 6, text: '}', intra_spans: [] },
      ],
      foldable_context: false,
    }],
  }],
  stats: { additions: 6, deletions: 0, unchanged: 0 },
  warnings: [],
  truncated: false,
};

/** A Unicode example: "café" → "cafe" with intraline span on the accent. */
const EXAMPLE_UNICODE: DiffResult = {
  files: [{
    left: 'menu.txt',
    right: 'menu.txt',
    hunks: [{
      old_range: { start: 1, len: 4 },
      new_range: { start: 1, len: 4 },
      lines: [
        { kind: 'context', old_no: 1, new_no: 1, text: 'drinks', intra_spans: [] },
        {
          kind: 'deletion', old_no: 2, new_no: null, text: 'café',
          intra_spans: [{ start: 3, len: 2 }],
        },
        {
          kind: 'addition', old_no: null, new_no: 2, text: 'cafe',
          intra_spans: [{ start: 3, len: 1 }],
        },
        { kind: 'context', old_no: 3, new_no: 3, text: 'latte', intra_spans: [] },
        { kind: 'context', old_no: 4, new_no: 4, text: 'espresso', intra_spans: [] },
      ],
      foldable_context: false,
    }],
  }],
  stats: { additions: 1, deletions: 1, unchanged: 3 },
  warnings: [],
  truncated: false,
};

// ── Demo metadata ──────────────────────────────────────────────────────

interface DemoExample {
  id: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ size?: string | number }>;
  diff: DiffResult;
  tags: string[];
}

const EXAMPLES: DemoExample[] = [
  {
    id: 'basic',
    title: 'Single-line change',
    description: 'A one-word replacement with character-level intraline highlighting.',
    icon: FileText,
    diff: EXAMPLE_BASIC,
    tags: ['intraline spans', 'unified'],
  },
  {
    id: 'multi-hunk',
    title: 'Multi-hunk diff',
    description: 'Two separate changes in one file, with foldable context between hunks.',
    icon: FileCode2,
    diff: EXAMPLE_MULTI_HUNK,
    tags: ['multi-hunk', 'context folding'],
  },
  {
    id: 'added',
    title: 'New file (all additions)',
    description: 'A brand-new file — every line shows as an addition.',
    icon: Plus,
    diff: EXAMPLE_ADDED,
    tags: ['addition-only', 'new file'],
  },
  {
    id: 'unicode',
    title: 'Unicode & accents',
    description: 'An accented character change with correct byte-offset intraline spans.',
    icon: FileJson,
    diff: EXAMPLE_UNICODE,
    tags: ['unicode', 'intraline spans'],
  },
];

// ── Component ──────────────────────────────────────────────────────────

export function DemoGallery() {
  const [activeId, setActiveId] = useState<string>(EXAMPLES[0].id);
  const active = EXAMPLES.find((e) => e.id === activeId)!;

  return (
    <div className="demo-gallery">
      {/* ── Sidebar with example list ─────────────────── */}
      <aside className="demo-sidebar">
        <div className="demo-sidebar-header">
          <Layers3 size={18} />
          <h2>Live Examples</h2>
        </div>
        <p className="demo-sidebar-subtitle">
          Explore diff-tool's rendering capabilities with these pre-built scenarios.
        </p>
        <nav className="demo-nav" aria-label="Demo examples">
          {EXAMPLES.map((example) => {
            const Icon = example.icon;
            const isActive = example.id === activeId;
            return (
              <button
                key={example.id}
                className={`demo-nav-item ${isActive ? 'active' : ''}`}
                onClick={() => setActiveId(example.id)}
                type="button"
              >
                <span className="demo-nav-icon">
                  <Icon size={16} />
                </span>
                <span className="demo-nav-text">
                  <span className="demo-nav-label">{example.title}</span>
                  <span className="demo-nav-desc">{example.description}</span>
                </span>
                {isActive && (
                  <span className="demo-nav-arrow">
                    <ChevronRight size={14} />
                  </span>
                )}
              </button>
            );
          })}
        </nav>
        <div className="demo-sidebar-footer">
          <span className="demo-sidebar-badge">
            <ArrowRightLeft size={12} />
            Powered by diff-core (Rust) + @pierre/diffs
          </span>
        </div>
      </aside>

      {/* ── Main content area ────────────────────────── */}
      <section className="demo-main">
        <header className="demo-header">
          <div className="demo-header-info">
            <h3>{active.title}</h3>
            <p>{active.description}</p>
          </div>
          <div className="demo-header-tags">
            {active.tags.map((tag) => (
              <span key={tag} className="demo-tag">{tag}</span>
            ))}
          </div>
          <div className="demo-header-stats">
            <span className="demo-stat-add">+{active.diff.stats.additions}</span>
            <span className="demo-stat-del">{'\u2212'}{active.diff.stats.deletions}</span>
            <span className="demo-stat-unchanged">{active.diff.stats.unchanged} unchanged</span>
          </div>
        </header>

        <div className="demo-render-area">
          <NativeDiffRenderer diff={active.diff} />
        </div>
      </section>
    </div>
  );
}
