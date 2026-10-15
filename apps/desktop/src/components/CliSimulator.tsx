import { useState } from 'react';
import {
  Terminal,
  Copy,
  Check,
  Zap,
  Cpu,
  Layers,
  ArrowRight,
  ExternalLink,
} from 'lucide-react';

interface CliScenario {
  id: string;
  command: string;
  title: string;
  tag: string;
  description: string;
  stdout: string;
  exitCode: number;
}

const SCENARIOS: CliScenario[] = [
  {
    id: 'diff-basic',
    command: 'dt diff left.txt right.txt',
    title: 'Text Diff (Human Output)',
    tag: 'Human Summary',
    description: 'Human-readable diff summary with hunk locations, changes, and intraline precision.',
    stdout: [
      'diff left.txt right.txt',
      '--- left.txt',
      '+++ right.txt',
      '@@ -1,3 +1,3 @@',
      ' alpha',
      '-welcome',
      '+hello',
      ' gamma',
      '',
      'Summary: 1 addition, 1 deletion, 2 unchanged lines',
    ].join('\n'),
    exitCode: 1,
  },
  {
    id: 'diff-histogram',
    command: 'dt diff left.txt right.txt --algorithm histogram',
    title: 'Histogram Diff Algorithm',
    tag: '1.94x Faster',
    description: 'High-speed Histogram diff algorithm via imara-diff for low-churn structured edits.',
    stdout: [
      'diff left.txt right.txt (algorithm: histogram)',
      '--- left.txt',
      '+++ right.txt',
      '@@ -10,7 +10,8 @@',
      ' pub fn process_batch(items: &[Item]) -> Result<(), Error> {',
      '     for item in items {',
      '-        validate_item(item)?;',
      '+        validate_item_fast(item)?;',
      '+        metric_record("processed");',
      '     }',
      '     Ok(())',
      ' }',
      '',
      'Summary: 2 additions, 1 deletion (0.18ms)',
    ].join('\n'),
    exitCode: 1,
  },
  {
    id: 'diff-syntax',
    command: 'dt diff parser_old.rs parser_new.rs --algorithm syntax',
    title: 'Tree-sitter Syntax-Aware Diff',
    tag: 'AST Semantic',
    description: 'Tree-sitter AST parser recognizes function renames, moved blocks, and parameter modifications without line churn.',
    stdout: [
      'diff parser_old.rs parser_new.rs (algorithm: syntax, depth: statement)',
      '[AST] Matched 48 function nodes across 2 files',
      '  ~ Function parse_tokens() -> Modified (signature & body changed)',
      '  + Function parse_ast_statement() -> Added',
      '  - Function legacy_scanner() -> Deleted',
      '  ⇄ Function normalize_whitespace() -> Moved (lines 45-62 -> 88-105)',
      '',
      '@@ -45,12 +88,12 @@ [Moved: normalize_whitespace]',
      ' pub fn normalize_whitespace(input: &str) -> String {',
      '     input.split_whitespace().collect::<Vec<_>>().join(" ")',
      ' }',
      '',
      'Summary: 1 modified, 1 added, 1 deleted, 1 moved AST node',
    ].join('\n'),
    exitCode: 1,
  },
  {
    id: 'diff-json',
    command: 'dt diff left.txt right.txt --json',
    title: 'Deterministic JSON AST Diff',
    tag: 'Machine Readable',
    description: 'Machine-parseable JSON contract with hunk coordinates and intraline byte-offset spans.',
    stdout: JSON.stringify(
      {
        files: [
          {
            left: 'left.txt',
            right: 'right.txt',
            hunks: [
              {
                old_range: { start: 1, len: 3 },
                new_range: { start: 1, len: 3 },
                lines: [
                  { kind: 'context', old_no: 1, new_no: 1, text: 'alpha', intra_spans: [] },
                  {
                    kind: 'deletion',
                    old_no: 2,
                    new_no: null,
                    text: 'welcome',
                    intra_spans: [{ start: 0, len: 7 }],
                  },
                  {
                    kind: 'addition',
                    old_no: null,
                    new_no: 2,
                    text: 'hello',
                    intra_spans: [{ start: 0, len: 5 }],
                  },
                  { kind: 'context', old_no: 3, new_no: 3, text: 'gamma', intra_spans: [] },
                ],
                foldable_context: false,
              },
            ],
          },
        ],
        stats: { additions: 1, deletions: 1, unchanged: 2 },
        warnings: [],
        truncated: false,
      },
      null,
      2,
    ),
    exitCode: 1,
  },
  {
    id: 'dir-scan',
    command: 'dt dir folder_a/ folder_b/',
    title: 'Directory Tree Comparison',
    tag: 'Folder Diff',
    description: 'High-throughput parallel tree walking with gitignore awareness and file status classification.',
    stdout: [
      'Scanning: folder_a/ ↔ folder_b/',
      '  M  Cargo.toml                     (modified, +8 -3)',
      '  +  docs/architecture.md           (added, 2.4 KiB)',
      '  M  src/App.tsx                    (modified, +14 -2)',
      '  -  temp_notes.txt                 (deleted)',
      '  M  crates/diff-core/src/lib.rs    (modified, +42 -10)',
      '',
      'Summary: 3 modified, 1 added, 1 deleted, 14 unchanged files (1.2ms)',
    ].join('\n'),
    exitCode: 1,
  },
  {
    id: 'ai-packages',
    command: 'dt ai list-packages',
    title: 'AI Prompt Packages',
    tag: 'Prompt Packs',
    description: 'Inspect available modular prompt packages with metadata, categories, and token budgets.',
    stdout: [
      'Available AI Prompt Packages:',
      '',
      '  full-review        Comprehensive Code Review      v1 (Review)',
      '    All-in-one report: executive summary, risk level, reviewer questions, and conventional commit suggestion.',
      '    Tags: review, summary, risk, questions',
      '',
      '  change-summary     Executive Change Summary       v1 (Review)',
      '    High-level summary of intent, bulleted changes, and notable risk signals.',
      '    Tags: summary, intent, tldr',
      '',
      '  risk-assessment    Risk & Blast Radius Audit      v1 (Audit)',
      '    Evaluates risk level (low/medium/high), justification, and affected modules or dependencies.',
      '    Tags: risk, security, impact, audit',
      '',
      '  review-questions   Targeted Reviewer Questions    v1 (Review)',
      '    Generates 3-5 critical questions focusing on edge cases, missing test fixtures, and security concerns.',
      '    Tags: questions, review, checklist',
      '',
      '  commit-message     Conventional Commit Generator  v1 (Authoring)',
      '    Generates a structured conventional commit message (feat, fix, refactor) from staged or file diffs.',
      '    Tags: commit, git, conventional',
      '',
      '  explain-hunk       Hunk / Line Change Explainer   v1 (Explain)',
      '    Explains a specific diff hunk or selected code change in plain English.',
      '    Tags: explain, hunk, education',
    ].join('\n'),
    exitCode: 0,
  },
  {
    id: 'ai-full-review',
    command: 'git diff | dt ai full-review',
    title: 'All-in-One AI Code Review',
    tag: 'One Round-Trip',
    description: 'Generates executive summary, risk level, checklist questions, and commit message in a single LLM call.',
    stdout: [
      '=== AI Code Review Report ===',
      '',
      'TL;DR: Migrates legacy ReactDOM.render calls to React 18 createRoot in test suites, eliminating synchronous unmount deprecation warnings.',
      '',
      'Key Intents:',
      '  • Modernize React test harness to use concurrent createRoot API',
      '  • Eliminate deprecation warnings in test logs',
      '  • Ensure consistent root teardown via root.unmount()',
      '',
      'Risk Rating: LOW',
      'Justification: Changes are strictly confined to test fixtures and assertions. Runtime package code remains untouched.',
      '',
      'Reviewer Questions:',
      '  1. Are there any lingering tests in the suite still relying on synchronous ReactDOM.render semantics?',
      '     Context: Check package-level CI logs for deprecation notices.',
      '  2. Does root.unmount() run reliably inside afterEach even when assertions fail mid-test?',
      '     Context: Prevents DOM leak across sequential test runs.',
      '',
      'Suggested Commit: test(react-dom): migrate renderer test harness to createRoot',
    ].join('\n'),
    exitCode: 0,
  },

  {
    id: 'help',
    command: 'dt --help',
    title: 'CLI Reference & Flags',
    tag: 'Manual',
    description: 'Full command-line reference showing subcommands, algorithms, formats, and deadline guards.',
    stdout: [
      'Diff Tool (dt) 0.1.0',
      'Cross-platform high-performance syntax-aware diff and merge engine.',
      '',
      'USAGE:',
      '    dt <SUBCOMMAND>',
      '',
      'SUBCOMMANDS:',
      '    diff      Compare two files (Myers, Histogram, or Tree-sitter Syntax)',
      '    dir       Compare two directories or scan a single tree',
      '    merge     Three-way merge resolution with conflict markers [WIP]',
      '    tui       Terminal user interface with split-pane navigation',
      '',
      'DIFF OPTIONS:',
      '    --algorithm <ALGO>      myers | histogram | syntax (default: myers)',
      '    --json                  Emit machine-parseable JSON contract',
      '    --ignore-whitespace     Ignore whitespace differences',
      '    --ignore-case           Ignore character case differences',
      '    --deadline-ms <MS>      Cancel diff if execution exceeds millisecond budget',
      '    --context <N>           Lines of surrounding context (default: 3)',
    ].join('\n'),
    exitCode: 0,
  },
];

export function CliSimulator() {
  const [activeScenarioId, setActiveScenarioId] = useState<string>(SCENARIOS[0].id);
  const [copiedCmd, setCopiedCmd] = useState(false);

  const active = SCENARIOS.find((s) => s.id === activeScenarioId) ?? SCENARIOS[0];

  const handleCopyCommand = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedCmd(true);
    setTimeout(() => setCopiedCmd(false), 2000);
  };

  return (
    <div className="cli-simulator-container">
      {/* ── Banner Section ──────────────────────────────── */}
      <section className="cli-banner">
        <div className="cli-banner-header">
          <Terminal size={24} className="accent-icon" />
          <h2>Command-Line Interface & Benchmarks</h2>
        </div>
        <p className="cli-banner-desc">
          The same zero-dependency Rust engine (<code>diff-core</code>) powers both the desktop GUI
          and the native <code>dt</code> CLI. Fast, deterministic, and scriptable.
        </p>

        <div className="cli-install-box">
          <span className="cli-install-label">Install via Cargo:</span>
          <code className="cli-install-code">cargo install --git https://github.com/chid/diff-tool diff-cli</code>
          <button
            className="cli-copy-btn"
            onClick={() =>
              handleCopyCommand('cargo install --git https://github.com/chid/diff-tool diff-cli')
            }
            title="Copy cargo install command"
            type="button"
          >
            {copiedCmd ? <Check size={14} color="#4ade80" /> : <Copy size={14} />}
            <span>{copiedCmd ? 'Copied' : 'Copy'}</span>
          </button>
        </div>
      </section>

      {/* ── Interactive Terminal ────────────────────────── */}
      <section className="cli-terminal-section">
        <div className="cli-scenario-chips">
          {SCENARIOS.map((s) => (
            <button
              key={s.id}
              className={`cli-chip ${s.id === activeScenarioId ? 'active' : ''}`}
              onClick={() => setActiveScenarioId(s.id)}
              type="button"
            >
              <span className="cli-chip-tag">{s.tag}</span>
              <span className="cli-chip-cmd">{s.command}</span>
            </button>
          ))}
        </div>

        <div className="terminal-window">
          <div className="terminal-window-header">
            <div className="terminal-window-dots">
              <span className="dot red" />
              <span className="dot yellow" />
              <span className="dot green" />
            </div>
            <span className="terminal-window-title">bash — dt (diff-tool)</span>
            <button
              className="terminal-copy-btn"
              onClick={() => handleCopyCommand(active.command)}
              title="Copy active command"
              type="button"
            >
              <Copy size={12} />
              <span>Copy</span>
            </button>
          </div>

          <div className="terminal-body">
            <div className="terminal-prompt-line">
              <span className="prompt-symbol">$</span>
              <span className="prompt-cmd">{active.command}</span>
            </div>
            <pre className="terminal-stdout">{active.stdout}</pre>
            <div className="terminal-footer-line">
              <span className="exit-code">Process exited with code {active.exitCode}</span>
            </div>
          </div>
        </div>
      </section>

      {/* ── Performance Benchmarks ──────────────────────── */}
      <section className="cli-benchmarks-section">
        <div className="benchmarks-header">
          <Zap size={20} className="accent-icon" />
          <h3>Performance Benchmarks</h3>
        </div>
        <p className="benchmarks-desc">
          Automated microbenchmarks measured on Apple Silicon using <code>scripts/measure-performance.mjs</code>.
        </p>

        <div className="benchmarks-grid">
          <div className="benchmark-card">
            <span className="benchmark-title">Sparse Edits (1,000 lines)</span>
            <span className="benchmark-stat">~2.7 ms</span>
            <span className="benchmark-sub">Sub-millisecond intraline character tokens</span>
          </div>

          <div className="benchmark-card">
            <span className="benchmark-title">Histogram Algorithm</span>
            <span className="benchmark-stat">1.94x</span>
            <span className="benchmark-sub">Speedup vs standard Myers algorithm</span>
          </div>

          <div className="benchmark-card">
            <span className="benchmark-title">Tree-sitter AST Diff</span>
            <span className="benchmark-stat">~4.0 ms</span>
            <span className="benchmark-sub">100 functions with statement-level tracking</span>
          </div>

          <div className="benchmark-card">
            <span className="benchmark-title">Directory Tree Scan</span>
            <span className="benchmark-stat">&gt;50k files/s</span>
            <span className="benchmark-sub">Parallel multi-threaded walk with gitignore</span>
          </div>
        </div>
      </section>

      {/* ── Architecture Highlights ─────────────────────── */}
      <section className="cli-arch-section">
        <div className="arch-header">
          <Cpu size={20} className="accent-icon" />
          <h3>Modular Architecture</h3>
        </div>

        <div className="arch-cards">
          <div className="arch-card">
            <h4>crates/diff-core</h4>
            <p>
              High-performance Rust core containing Myers, Histogram, and Tree-sitter syntax diffing algorithms,
              intraline byte-offset token spans, and UTF-8 validation.
            </p>
          </div>

          <div className="arch-card">
            <h4>crates/git-adapter</h4>
            <p>
              Filesystem traversal engine for single-root and folder-to-folder comparisons, respecting gitignore
              and file status tracking.
            </p>
          </div>

          <div className="arch-card">
            <h4>apps/desktop</h4>
            <p>
              Tauri v2 native desktop application shell paired with @pierre/diffs and React for smooth 60fps
              virtualized diff rendering.
            </p>
          </div>

          <div className="arch-card">
            <h4>crates/diff-cli</h4>
            <p>
              Command-line executable (<code>dt</code>) providing colored terminal outputs, progress bars,
              and machine-readable JSON schemas for IDE integrations.
            </p>
          </div>
        </div>
      </section>
    </div>
  );
}
