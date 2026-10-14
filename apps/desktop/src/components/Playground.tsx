import { useState, useMemo, useRef, useCallback } from 'react';
import { parseDiffFromFile } from '@pierre/diffs';
import { FileDiff } from '@pierre/diffs/react';
import type { FileDiffOptions } from '@pierre/diffs';
import {
  ArrowLeftRight,
  RotateCcw,
  Upload,
  FileCode2,
  Sparkles,
  Copy,
  Check,
  SplitSquareHorizontal,
} from 'lucide-react';
import { useSettings } from './SettingsContext';
import { settingsToFileDiffOptions } from '../settingsTypes';

interface Preset {
  id: string;
  name: string;
  lang: string;
  filename: string;
  left: string;
  right: string;
  description: string;
}

const PRESETS: Preset[] = [
  {
    id: 'ts-hook',
    name: 'TypeScript: React Hook Refactor',
    lang: 'typescript',
    filename: 'useCounter.ts',
    description: 'Refactoring state to support bounded limits with step options.',
    left: [
      'import { useState } from "react";',
      '',
      'export function useCounter(initialValue: number = 0) {',
      '  const [count, setCount] = useState<number>(initialValue);',
      '',
      '  const increment = () => setCount((c) => c + 1);',
      '  const decrement = () => setCount((c) => c - 1);',
      '  const reset = () => setCount(initialValue);',
      '',
      '  return { count, increment, decrement, reset };',
      '}',
    ].join('\n'),
    right: [
      'import { useState, useCallback } from "react";',
      '',
      'export interface CounterOptions {',
      '  min?: number;',
      '  max?: number;',
      '  step?: number;',
      '}',
      '',
      'export function useCounter(initial: number = 0, options: CounterOptions = {}) {',
      '  const { min = -Infinity, max = Infinity, step = 1 } = options;',
      '  const [count, setCount] = useState<number>(() => Math.min(max, Math.max(min, initial)));',
      '',
      '  const increment = useCallback(() => {',
      '    setCount((c) => Math.min(max, c + step));',
      '  }, [max, step]);',
      '',
      '  const decrement = useCallback(() => {',
      '    setCount((c) => Math.max(min, c - step));',
      '  }, [min, step]);',
      '',
      '  const reset = useCallback(() => {',
      '    setCount(Math.min(max, Math.max(min, initial)));',
      '  }, [initial, max, min]);',
      '',
      '  return { count, increment, decrement, reset };',
      '}',
    ].join('\n'),
  },
  {
    id: 'rust-match',
    name: 'Rust: Tree-sitter Syntax Diff',
    lang: 'rust',
    filename: 'algorithm.rs',
    description: 'Adding Histogram and AST-aware Syntax diff algorithms with error handling.',
    left: [
      '#[derive(Debug, Clone, Copy, PartialEq, Eq)]',
      'pub enum DiffAlgorithm {',
      '    Myers,',
      '}',
      '',
      'pub fn compute_diff(a: &str, b: &str) -> Result<DiffResult, DiffError> {',
      '    let result = imara_diff::diff(a, b);',
      '    Ok(result)',
      '}',
    ].join('\n'),
    right: [
      '#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]',
      'pub enum DiffAlgorithm {',
      '    #[default]',
      '    Myers,',
      '    Histogram,',
      '    Syntax,',
      '}',
      '',
      'pub fn compute_diff(a: &str, b: &str, algo: DiffAlgorithm) -> Result<DiffResult, DiffError> {',
      '    match algo {',
      '        DiffAlgorithm::Myers => Ok(imara_diff::myers(a, b)),',
      '        DiffAlgorithm::Histogram => Ok(imara_diff::histogram(a, b)),',
      '        DiffAlgorithm::Syntax => {',
      '            let ast_diff = parser::syntax_diff(a, b)?;',
      '            Ok(ast_diff.into_diff_result())',
      '        }',
      '    }',
      '}',
    ].join('\n'),
  },
  {
    id: 'python-async',
    name: 'Python: Async Pipeline',
    lang: 'python',
    filename: 'fetcher.py',
    description: 'Upgrading requests to httpx with batch concurrency and retry exponential backoff.',
    left: [
      'import requests',
      '',
      'def fetch_item(item_id: int) -> dict:',
      '    url = f"https://api.example.com/items/{item_id}"',
      '    response = requests.get(url, timeout=5)',
      '    response.raise_for_status()',
      '    return response.json()',
    ].join('\n'),
    right: [
      'import asyncio',
      'import httpx',
      'from typing import Any',
      '',
      'async def fetch_item(client: httpx.AsyncClient, item_id: int, retries: int = 3) -> dict[str, Any]:',
      '    url = f"https://api.example.com/items/{item_id}"',
      '    for attempt in range(retries):',
      '        try:',
      '            resp = await client.get(url, timeout=5.0)',
      '            resp.raise_for_status()',
      '            return resp.json()',
      '        except httpx.HTTPError:',
      '            if attempt == retries - 1:',
      '                raise',
      '            await asyncio.sleep(2 ** attempt)',
    ].join('\n'),
  },
  {
    id: 'intraline-precision',
    name: 'Intraline: Character-Level Edits',
    lang: 'text',
    filename: 'config.env',
    description: 'Pinpointing precise single-word and character token modifications.',
    left: [
      'DATABASE_URL=postgres://charley:old_secret_pwd@127.0.0.1:5432/primary_db',
      'CACHE_TTL_SECONDS=3600',
      'ENABLE_TELEMETRY=true',
      'WELCOME_MESSAGE="Welcome to Diff Tool version 0.1"',
    ].join('\n'),
    right: [
      'DATABASE_URL=postgres://charley:super_secure_vault_pass@127.0.0.1:5432/primary_db',
      'CACHE_TTL_SECONDS=7200',
      'ENABLE_TELEMETRY=false',
      'WELCOME_MESSAGE="Hello to Diff Tool version 0.2"',
    ].join('\n'),
  },
];

export function Playground() {
  const { settings, updateSetting } = useSettings();
  const [selectedPresetId, setSelectedPresetId] = useState<string>(PRESETS[0].id);
  const [filename, setFilename] = useState<string>(PRESETS[0].filename);
  const [leftText, setLeftText] = useState<string>(PRESETS[0].left);
  const [rightText, setRightText] = useState<string>(PRESETS[0].right);
  const [copied, setCopied] = useState(false);

  const leftInputRef = useRef<HTMLInputElement>(null);
  const rightInputRef = useRef<HTMLInputElement>(null);

  const handleSelectPreset = (id: string) => {
    setSelectedPresetId(id);
    const p = PRESETS.find((preset) => preset.id === id);
    if (p) {
      setFilename(p.filename);
      setLeftText(p.left);
      setRightText(p.right);
    }
  };

  const handleSwap = () => {
    setLeftText(rightText);
    setRightText(leftText);
  };

  const handleReset = () => {
    const p = PRESETS.find((preset) => preset.id === selectedPresetId) ?? PRESETS[0];
    setLeftText(p.left);
    setRightText(p.right);
    setFilename(p.filename);
  };

  const handleLeftUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const text = await file.text();
      setLeftText(text);
      setFilename(file.name);
    }
  };

  const handleRightUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const text = await file.text();
      setRightText(text);
      setFilename(file.name);
    }
  };

  const fileDiff = useMemo(() => {
    return parseDiffFromFile(
      { name: filename, contents: leftText },
      { name: filename, contents: rightText },
    );
  }, [filename, leftText, rightText]);

  const stats = useMemo(() => {
    let additions = 0;
    let deletions = 0;
    let unchanged = 0;
    for (const hunk of fileDiff.hunks) {
      additions += hunk.additionLines;
      deletions += hunk.deletionLines;
      unchanged += Math.max(0, hunk.splitLineCount - hunk.additionLines);
    }
    return { additions, deletions, unchanged };
  }, [fileDiff]);

  const handleCopyUnified = () => {
    const lines: string[] = [`--- a/${filename}`, `+++ b/${filename}`];
    for (const hunk of fileDiff.hunks) {
      if (hunk.hunkSpecs) lines.push(hunk.hunkSpecs);
    }
    if (fileDiff.deletionLines) {
      for (const line of fileDiff.deletionLines) {
        lines.push(`-${line.replace(/\n$/, '')}`);
      }
    }
    if (fileDiff.additionLines) {
      for (const line of fileDiff.additionLines) {
        lines.push(`+${line.replace(/\n$/, '')}`);
      }
    }
    navigator.clipboard.writeText(lines.join('\n'));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="playground-container">
      {/* ── Hidden file inputs ──────────────────────────── */}
      <input
        type="file"
        ref={leftInputRef}
        onChange={handleLeftUpload}
        style={{ display: 'none' }}
      />
      <input
        type="file"
        ref={rightInputRef}
        onChange={handleRightUpload}
        style={{ display: 'none' }}
      />

      {/* ── Playground Header & Controls ───────────────── */}
      <header className="playground-toolbar">
        <div className="playground-presets">
          <label className="playground-label">
            <Sparkles size={14} className="accent-icon" />
            <span>Preset:</span>
          </label>
          <select
            className="playground-select"
            value={selectedPresetId}
            onChange={(e) => handleSelectPreset(e.target.value)}
          >
            {PRESETS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <input
            className="playground-filename-input"
            value={filename}
            onChange={(e) => setFilename(e.target.value)}
            placeholder="filename.ext"
            title="File name (used for syntax highlighter language detection)"
          />
        </div>

        <div className="playground-actions">
          <button
            className="playground-btn"
            onClick={handleSwap}
            title="Swap Original ↔ Modified"
            type="button"
          >
            <ArrowLeftRight size={14} />
            <span>Swap</span>
          </button>
          <button
            className="playground-btn"
            onClick={handleReset}
            title="Reset to selected preset"
            type="button"
          >
            <RotateCcw size={14} />
            <span>Reset</span>
          </button>
          <button
            className="playground-btn"
            onClick={handleCopyUnified}
            title="Copy diff patch to clipboard"
            type="button"
          >
            {copied ? <Check size={14} color="#4ade80" /> : <Copy size={14} />}
            <span>{copied ? 'Copied!' : 'Copy patch'}</span>
          </button>
          <div className="playground-style-toggles">
            <button
              className={settings.diffStyle === 'split' ? 'active' : ''}
              onClick={() => updateSetting('diffStyle', 'split')}
              type="button"
            >
              Split
            </button>
            <button
              className={settings.diffStyle === 'unified' ? 'active' : ''}
              onClick={() => updateSetting('diffStyle', 'unified')}
              type="button"
            >
              Unified
            </button>
          </div>
        </div>
      </header>

      {/* ── Input Editors ──────────────────────────────── */}
      <div className="playground-inputs-row">
        <div className="playground-editor-box">
          <div className="playground-editor-header">
            <span className="editor-title">
              <span className="editor-indicator left-indicator" />
              Original (Left)
            </span>
            <button
              className="editor-upload-btn"
              onClick={() => leftInputRef.current?.click()}
              title="Upload file for left side"
              type="button"
            >
              <Upload size={13} />
              <span>Upload file</span>
            </button>
          </div>
          <textarea
            className="playground-textarea"
            value={leftText}
            onChange={(e) => setLeftText(e.target.value)}
            placeholder="Paste or type original code here..."
            spellCheck={false}
          />
        </div>

        <div className="playground-editor-box">
          <div className="playground-editor-header">
            <span className="editor-title">
              <span className="editor-indicator right-indicator" />
              Modified (Right)
            </span>
            <button
              className="editor-upload-btn"
              onClick={() => rightInputRef.current?.click()}
              title="Upload file for right side"
              type="button"
            >
              <Upload size={13} />
              <span>Upload file</span>
            </button>
          </div>
          <textarea
            className="playground-textarea"
            value={rightText}
            onChange={(e) => setRightText(e.target.value)}
            placeholder="Paste or type modified code here..."
            spellCheck={false}
          />
        </div>
      </div>

      {/* ── Diff Result Header ─────────────────────────── */}
      <div className="playground-diff-header">
        <div className="playground-diff-info">
          <FileCode2 size={16} />
          <span className="playground-diff-title">{filename}</span>
          <span className="diff-stats-badge">
            <span className="stat-add">+{stats.additions}</span>
            <span className="stat-del">−{stats.deletions}</span>
            <span className="stat-same">{stats.unchanged} unchanged</span>
          </span>
        </div>
        <span className="playground-hint">
          Live rendered with syntax highlighting and intraline character tokens
        </span>
      </div>

      {/* ── Pierre Diff View ───────────────────────────── */}
      <div className="playground-diff-view">
        <FileDiff
          fileDiff={fileDiff}
          disableWorkerPool={settings.disableWorkerPool}
          options={settingsToFileDiffOptions(settings) as FileDiffOptions<undefined>}
          style={{ minHeight: 0, flex: '1 1 auto' }}
        />
      </div>
    </div>
  );
}
