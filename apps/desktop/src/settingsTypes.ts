import registryData from './settings-registry.json';

import type {
  DiffsThemeNames,
  DiffIndicators,
  HunkSeparators,
  LineDiffTypes,
  HighlighterTypes,
} from '@pierre/diffs';

// ─── JSON-driven runtime data ──────────────────────────────────────────────

/** Category metadata — loaded directly from settings-registry.json */
export interface CategoryMeta {
  key: string;
  label: string;
  description: string;
}

export const CATEGORIES: CategoryMeta[] = registryData.categories;

// ─── Per-setting metadata for the UI ───────────────────────────────────────

export type SettingControlType = 'toggle' | 'select' | 'number' | 'text' | 'password';

export interface SettingMeta {
  key: string;
  label: string;
  description: string;
  category: string;
  control: SettingControlType;
  default: string | number | boolean;
  placeholder?: string;
  options?: { label: string; value: string }[];
  min?: number;
  max?: number;
}

/**
 * Full settings registry — loaded from settings-registry.json.
 * We cast `as SettingMeta[]` because the JSON `control` field is widened to
 * `string`; the JSON is the canoncial source and must use valid control types.
 */
export const SETTINGS_REGISTRY: SettingMeta[] =
  registryData.settings as SettingMeta[];

// ─── Full Settings State (explicit interface for precise TypeScript types) ─

// NOTE: Keep this union in sync with the categories array in settings-registry.json.
// TypeScript can't derive precise literal types from JSON imports (they widen to string),
// so the type must be maintained manually alongside the JSON.
export type SettingsCategory = 'display' | 'algorithm' | 'performance' | 'interaction' | 'file' | 'ai';

export interface SettingsState {
  // Display
  diffStyle: 'unified' | 'split';
  theme: DiffsThemeNames;
  themeType: 'system' | 'light' | 'dark';
  overflow: 'scroll' | 'wrap';

  // Algorithm
  diffAlgorithm: 'myers' | 'histogram' | 'patience';
  defaultDeadline: number;
  diffIndicators: DiffIndicators;
  hunkSeparators: Exclude<HunkSeparators, 'custom'>;
  lineDiffType: LineDiffTypes;
  expandUnchanged: boolean;
  collapsedContextThreshold: number;
  expansionLineCount: number;

  // Performance
  disableWorkerPool: boolean;
  preferredHighlighter: HighlighterTypes;
  disableVirtualizationBuffers: boolean;
  tokenizeMaxLineLength: number;

  // Interaction
  lineHoverHighlight: 'disabled' | 'both' | 'number' | 'line';
  enableLineSelection: boolean;
  stickyHeader: boolean;
  enableTokenInteractionsOnWhitespace: boolean;

  // File Display
  disableLineNumbers: boolean;
  disableFileHeader: boolean;
  disableBackground: boolean;

  // AI
  aiProvider: 'openai' | 'anthropic' | 'custom';
  aiApiKey: string;
  aiModel: string;
  aiBaseUrl: string;
}

// ─── Defaults (derived from JSON at import time) ───────────────────────────

function buildDefaults(registry: SettingMeta[]): SettingsState {
  const defaults: Record<string, unknown> = {};
  for (const s of registry) {
    defaults[s.key] = s.default;
  }
  return defaults as unknown as SettingsState;
}

export const SETTINGS_DEFAULTS: SettingsState = buildDefaults(SETTINGS_REGISTRY);

// ─── Helper: extract @pierre/diffs FileDiff options ────────────────────────

export function settingsToFileDiffOptions(s: SettingsState) {
  return {
    // Core diff algorithm settings — passed through to the Rust backend
    diffAlgorithm: s.diffAlgorithm,
    defaultDeadline: s.defaultDeadline,
    // Display
    diffStyle: s.diffStyle,
    theme: s.theme,
    themeType: s.themeType,
    overflow: s.overflow,
    diffIndicators: s.diffIndicators,
    hunkSeparators: s.hunkSeparators,
    lineDiffType: s.lineDiffType,
    expandUnchanged: s.expandUnchanged,
    collapsedContextThreshold: s.collapsedContextThreshold,
    expansionLineCount: s.expansionLineCount,
    preferredHighlighter: s.preferredHighlighter,
    disableVirtualizationBuffers: s.disableVirtualizationBuffers,
    tokenizeMaxLineLength: s.tokenizeMaxLineLength,
    stickyHeader: s.stickyHeader,
    disableLineNumbers: s.disableLineNumbers,
    disableFileHeader: s.disableFileHeader,
    disableBackground: s.disableBackground,
    // InteractionManagerBaseOptions
    lineHoverHighlight: s.lineHoverHighlight,
    enableLineSelection: s.enableLineSelection,
    enableTokenInteractionsOnWhitespace: s.enableTokenInteractionsOnWhitespace,
  };
}
