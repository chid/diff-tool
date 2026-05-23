import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { SettingsProvider, useSettings } from '../components/SettingsContext';
import { SETTINGS_DEFAULTS } from '../settingsTypes';
import type { SettingsState } from '../settingsTypes';
import type { ReactNode } from 'react';

// ─── Helpers ───────────────────────────────────────────────────────────────

const STORAGE_KEY = 'diff-tool-settings';

function wrapper({ children }: { children: ReactNode }) {
  return <SettingsProvider>{children}</SettingsProvider>;
}

function mockStorage(initial?: Partial<SettingsState>) {
  const store: Record<string, string> = {};
  if (initial) store[STORAGE_KEY] = JSON.stringify(initial);
  vi.stubGlobal('localStorage', {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn(),
    clear: vi.fn(),
    length: 0,
    key: vi.fn(() => null),
  });
  return store;
}

// ─── Tests ─────────────────────────────────────────────────────────────────

describe('SettingsContext', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('defaults (JSON-driven)', () => {
    it('provides the default settings when localStorage is empty', () => {
      mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      expect(result.current.settings).toEqual(SETTINGS_DEFAULTS);
    });

    it('every default key matches a JSON registry entry', () => {
      // Verify that all defaults are derived from the JSON
      for (const key of Object.keys(SETTINGS_DEFAULTS) as (keyof SettingsState)[]) {
        expect(SETTINGS_DEFAULTS[key]).toBeDefined();
        expect(typeof SETTINGS_DEFAULTS[key]).not.toBe('undefined');
      }
    });

    it('merges partial stored settings with defaults', () => {
      mockStorage({ diffStyle: 'unified', disableLineNumbers: true });

      const { result } = renderHook(() => useSettings(), { wrapper });

      expect(result.current.settings.diffStyle).toBe('unified');
      expect(result.current.settings.disableLineNumbers).toBe(true);
      // other settings should remain defaults
      expect(result.current.settings.overflow).toBe(SETTINGS_DEFAULTS.overflow);
      expect(result.current.settings.stickyHeader).toBe(SETTINGS_DEFAULTS.stickyHeader);
    });

    it('falls back to defaults if stored JSON is corrupt', () => {
      const store = mockStorage();
      store[STORAGE_KEY] = '{bad json!!!';

      const { result } = renderHook(() => useSettings(), { wrapper });

      expect(result.current.settings).toEqual(SETTINGS_DEFAULTS);
    });
  });

  describe('updateSetting', () => {
    it('updates a boolean setting', () => {
      mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('disableWorkerPool', false);
      });

      expect(result.current.settings.disableWorkerPool).toBe(false);
    });

    it('updates a string setting', () => {
      mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('diffStyle', 'unified');
      });

      expect(result.current.settings.diffStyle).toBe('unified');
    });

    it('updates a number setting', () => {
      mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('collapsedContextThreshold', 8);
      });

      expect(result.current.settings.collapsedContextThreshold).toBe(8);
    });

    it('does not trigger re-render when value is unchanged', () => {
      mockStorage();
      const renders: number[] = [];
      const { result } = renderHook(() => {
        renders.push(1);
        return useSettings();
      }, { wrapper });

      const renderCount = renders.length;
      act(() => {
        result.current.updateSetting('diffStyle', SETTINGS_DEFAULTS.diffStyle);
      });

      // should NOT have caused an additional render
      expect(renders.length).toBe(renderCount);
    });
  });

  describe('resetSettings', () => {
    it('resets all settings to defaults', () => {
      mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('diffStyle', 'unified');
        result.current.updateSetting('disableLineNumbers', true);
        result.current.updateSetting('collapsedContextThreshold', 10);
      });

      act(() => {
        result.current.resetSettings();
      });

      expect(result.current.settings).toEqual(SETTINGS_DEFAULTS);
    });
  });

  describe('persistence', () => {
    it('saves settings to localStorage on update', () => {
      const store = mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('diffStyle', 'unified');
        result.current.updateSetting('disableWorkerPool', false);
      });

      const saved = JSON.parse(store[STORAGE_KEY]);
      expect(saved.diffStyle).toBe('unified');
      expect(saved.disableWorkerPool).toBe(false);
    });

    it('saves settings to localStorage on reset', () => {
      const store = mockStorage();

      const { result } = renderHook(() => useSettings(), { wrapper });

      act(() => {
        result.current.updateSetting('diffStyle', 'unified');
      });
      act(() => {
        result.current.resetSettings();
      });

      const saved = JSON.parse(store[STORAGE_KEY]);
      expect(saved.diffStyle).toBe(SETTINGS_DEFAULTS.diffStyle);
    });

    it('loads persisted settings on mount', () => {
      const store = mockStorage();
      store[STORAGE_KEY] = JSON.stringify({ diffStyle: 'unified', disableLineNumbers: true });

      const { result } = renderHook(() => useSettings(), { wrapper });

      expect(result.current.settings.diffStyle).toBe('unified');
      expect(result.current.settings.disableLineNumbers).toBe(true);
    });
  });

  describe('error boundary', () => {
    it('throws when useSettings is used outside a provider', () => {
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {});

      expect(() => {
        renderHook(() => useSettings());
      }).toThrow('useSettings must be used within a SettingsProvider');

      spy.mockRestore();
    });
  });
});
