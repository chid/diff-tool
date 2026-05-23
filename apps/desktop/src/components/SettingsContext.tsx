import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import type { SettingsState } from '../settingsTypes';
import { SETTINGS_DEFAULTS } from '../settingsTypes';

const STORAGE_KEY = 'diff-tool-settings';

// ─── Context ───────────────────────────────────────────────────────────────

interface SettingsContextValue {
  settings: SettingsState;
  updateSetting: <K extends keyof SettingsState>(key: K, value: SettingsState[K]) => void;
  resetSettings: () => void;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

// ─── Helpers ───────────────────────────────────────────────────────────────

function loadSettings(): SettingsState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SettingsState>;
      return { ...SETTINGS_DEFAULTS, ...parsed };
    }
  } catch {
    // corrupted storage — fall through to defaults
  }
  return { ...SETTINGS_DEFAULTS };
}

function saveSettings(s: SettingsState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch {
    // storage full or unavailable — silently ignore
  }
}

// ─── Provider ──────────────────────────────────────────────────────────────

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<SettingsState>(loadSettings);

  // Persist to localStorage whenever settings change
  useEffect(() => {
    saveSettings(settings);
  }, [settings]);

  const updateSetting = useCallback(
    <K extends keyof SettingsState>(key: K, value: SettingsState[K]) => {
      setSettings((prev) => {
        if (prev[key] === value) return prev;
        return { ...prev, [key]: value };
      });
    },
    [],
  );

  const resetSettings = useCallback(() => {
    setSettings({ ...SETTINGS_DEFAULTS });
  }, []);

  return (
    <SettingsContext.Provider value={{ settings, updateSetting, resetSettings }}>
      {children}
    </SettingsContext.Provider>
  );
}

// ─── Hook ──────────────────────────────────────────────────────────────────

export function useSettings(): SettingsContextValue {
  const ctx = useContext(SettingsContext);
  if (!ctx) {
    throw new Error('useSettings must be used within a SettingsProvider');
  }
  return ctx;
}
