import { useState, useRef, useEffect, useMemo } from 'react';
import {
  Settings,
  RotateCcw,
  Palette,
  GitBranch,
  Cpu,
  MousePointer,
  File,
  Brain,
  EyeOff,
  Eye,
  Dot,
} from 'lucide-react';
import { useSettings } from './SettingsContext';
import {
  CATEGORIES,
  SETTINGS_REGISTRY,
  SETTINGS_DEFAULTS,
  type SettingsCategory,
  type SettingMeta,
  type SettingControlType,
} from '../settingsTypes';

// ─── Category icons (from lucide-react) ────────────────────────────────────

const CATEGORY_ICONS: Record<string, React.ComponentType<any>> = {
  display: Palette,
  algorithm: GitBranch,
  performance: Cpu,
  interaction: MousePointer,
  file: File,
  ai: Brain,
};

// ─── Control renderers ─────────────────────────────────────────────────────

function SettingToggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className={`setting-toggle ${checked ? 'on' : 'off'}`}
      onClick={() => onChange(!checked)}
      type="button"
    >
      <span className="toggle-track">
        <span className="toggle-thumb" />
      </span>
    </button>
  );
}

function SettingSelect({
  value,
  options,
  onChange,
  label,
}: {
  value: string;
  options: { label: string; value: string }[];
  onChange: (v: string) => void;
  label: string;
}) {
  return (
    <select
      className="setting-select"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      aria-label={label}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}

function SettingNumber({
  value,
  min,
  max,
  onChange,
  label,
}: {
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
  label: string;
}) {
  const [local, setLocal] = useState(String(value));
  const timer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    setLocal(String(value));
  }, [value]);

  const commit = (raw: string) => {
    const n = Number(raw);
    if (!Number.isNaN(n)) {
      const clamped = Math.max(min, Math.min(max, n));
      onChange(clamped);
      setLocal(String(clamped));
    } else {
      setLocal(String(value));
    }
  };

  return (
    <input
      type="number"
      className="setting-number"
      value={local}
      min={min}
      max={max}
      aria-label={label}
      onChange={(e) => {
        setLocal(e.target.value);
        clearTimeout(timer.current);
        timer.current = setTimeout(() => commit(e.target.value), 600);
      }}
      onBlur={(e) => {
        clearTimeout(timer.current);
        commit(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          clearTimeout(timer.current);
          commit(e.currentTarget.value);
        }
      }}
    />
  );
}

function SettingText({
  value,
  placeholder,
  onChange,
  label,
}: {
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
  label: string;
}) {
  const [local, setLocal] = useState(value);
  const timer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    setLocal(value);
  }, [value]);

  const commit = (raw: string) => {
    onChange(raw);
  };

  return (
    <input
      type="text"
      className="setting-text"
      value={local}
      placeholder={placeholder}
      aria-label={label}
      onChange={(e) => {
        setLocal(e.target.value);
        clearTimeout(timer.current);
        timer.current = setTimeout(() => commit(e.target.value), 400);
      }}
      onBlur={(e) => {
        clearTimeout(timer.current);
        commit(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          clearTimeout(timer.current);
          commit(e.currentTarget.value);
        }
      }}
    />
  );
}

function SettingPassword({
  value,
  placeholder,
  onChange,
  label,
}: {
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
  label: string;
}) {
  const [local, setLocal] = useState(value);
  const [visible, setVisible] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    setLocal(value);
  }, [value]);

  const commit = (raw: string) => {
    onChange(raw);
  };

  return (
    <div className="setting-password-wrap">
      <input
        type={visible ? 'text' : 'password'}
        className="setting-text"
        value={local}
        placeholder={placeholder}
        aria-label={label}
        onChange={(e) => {
          setLocal(e.target.value);
          clearTimeout(timer.current);
          timer.current = setTimeout(() => commit(e.target.value), 400);
        }}
        onBlur={(e) => {
          clearTimeout(timer.current);
          commit(e.target.value);
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            clearTimeout(timer.current);
            commit(e.currentTarget.value);
          }
        }}
      />
      <button
        type="button"
        className="setting-password-toggle"
        onClick={() => setVisible((v) => !v)}
        aria-label={visible ? 'Hide API key' : 'Show API key'}
        title={visible ? 'Hide API key' : 'Show API key'}
      >
        {visible ? <EyeOff size={14} /> : <Eye size={14} />}
      </button>
    </div>
  );
}

function SettingControl({
  value,
  meta,
  onChange,
}: {
  value: unknown;
  meta: SettingMeta;
  onChange: (v: unknown) => void;
}) {
  switch (meta.control) {
    case 'toggle':
      return (
        <SettingToggle
          checked={value as boolean}
          onChange={onChange as (v: boolean) => void}
          label={meta.label}
        />
      );
    case 'select':
      return (
        <SettingSelect
          value={value as string}
          options={meta.options as { label: string; value: string }[]}
          onChange={onChange as (v: string) => void}
          label={meta.label}
        />
      );
    case 'number':
      return (
        <SettingNumber
          value={value as number}
          min={meta.min ?? 0}
          max={meta.max ?? 100}
          onChange={onChange as (v: number) => void}
          label={meta.label}
        />
      );
    case 'text':
      return (
        <SettingText
          value={value as string}
          placeholder={meta.placeholder}
          onChange={onChange as (v: string) => void}
          label={meta.label}
        />
      );
    case 'password':
      return (
        <SettingPassword
          value={value as string}
          placeholder={meta.placeholder}
          onChange={onChange as (v: string) => void}
          label={meta.label}
        />
      );
    default:
      return null;
  }
}

// ─── Helpers ───────────────────────────────────────────────────────────────

/** Check if a setting value differs from its default */
function isModified(key: string, value: unknown): boolean {
  const def = SETTINGS_DEFAULTS[key as keyof typeof SETTINGS_DEFAULTS];
  return value !== def;
}

// ─── Panel ─────────────────────────────────────────────────────────────────

export function SettingsPanel() {
  const { settings, updateSetting, resetSettings } = useSettings();
  const [open, setOpen] = useState(false);
  const [activeCategory, setActiveCategory] = useState<SettingsCategory>('display');
  const [animatingOut, setAnimatingOut] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(true);
  const closingRef = useRef(false);

  // Track mounted state for timeout cleanup
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Compute modified counts per category
  const modifiedPerCategory = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const cat of CATEGORIES) {
      counts[cat.key] = SETTINGS_REGISTRY.filter(
        (m) => m.category === cat.key && isModified(m.key, settings[m.key as keyof typeof settings]),
      ).length;
    }
    return counts;
  }, [settings]);

  const totalModified = useMemo(
    () => Object.values(modifiedPerCategory).reduce((a, b) => a + b, 0),
    [modifiedPerCategory],
  );

  // Close with fade animation — guards against double-close during animation
  const close = () => {
    if (closingRef.current) return;
    closingRef.current = true;
    setAnimatingOut(true);
    setTimeout(() => {
      if (!mountedRef.current) return;
      setOpen(false);
      setAnimatingOut(false);
      closingRef.current = false;
    }, 180);
  };

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [open]);

  // Focus trap
  useEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    if (!panel) return;

    const focusable = panel.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    );
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    const handler = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
      }
    };

    first?.focus();
    panel.addEventListener('keydown', handler);
    return () => panel.removeEventListener('keydown', handler);
  }, [open, activeCategory]);

  const settingsForCategory = SETTINGS_REGISTRY.filter((m) => m.category === activeCategory);
  const currentCategory = CATEGORIES.find((c) => c.key === activeCategory);
  const CategoryIcon = currentCategory ? CATEGORY_ICONS[currentCategory.key] : null;

  return (
    <>
      {/* Trigger button */}
      <button
        className="settings-trigger"
        aria-label={`Open settings${totalModified > 0 ? ` (${totalModified} modified)` : ''}`}
        title={`Settings${totalModified > 0 ? ` — ${totalModified} modified` : ''}`}
        onClick={() => {
          closingRef.current = false;
          setAnimatingOut(false);
          setOpen(true);
        }}
        type="button"
      >
        <Settings size={18} />
        {totalModified > 0 && <span className="settings-trigger-badge">{totalModified}</span>}
      </button>

      {open && (
        <div
          className={`settings-overlay ${animatingOut ? 'fade-out' : 'fade-in'}`}
          role="dialog"
          aria-modal="true"
          aria-label="Settings"
          onClick={(e) => {
            if (e.target === e.currentTarget) close();
          }}
        >
          <div className={`settings-panel ${animatingOut ? 'slide-out' : 'slide-in'}`} ref={panelRef}>
            {/* ── Sidebar ──────────────────────────────── */}
            <div className="settings-sidebar">
              <div className="settings-sidebar-header">
                <h2 className="settings-title">Preferences</h2>
                <span className="settings-subtitle">Configure your diff experience</span>
              </div>

              <nav className="settings-nav" aria-label="Settings categories">
                {CATEGORIES.map((cat) => {
                  const Icon = CATEGORY_ICONS[cat.key];
                  const modified = modifiedPerCategory[cat.key] ?? 0;
                  return (
                    <button
                      key={cat.key}
                      className={`settings-nav-item ${cat.key === activeCategory ? 'active' : ''}`}
                      onClick={() => setActiveCategory(cat.key as SettingsCategory)}
                      type="button"
                    >
                      <span className="settings-nav-icon">
                        {Icon && <Icon size={16} />}
                      </span>
                      <span className="settings-nav-text">
                        <span className="settings-nav-label">{cat.label}</span>
                        <span className="settings-nav-desc">{cat.description}</span>
                      </span>
                      {modified > 0 && (
                        <span className="settings-nav-badge">{modified}</span>
                      )}
                    </button>
                  );
                })}
              </nav>

              <button
                className="settings-reset"
                onClick={() => {
                  if (window.confirm('Reset all settings to their defaults?')) {
                    resetSettings();
                  }
                }}
                type="button"
              >
                <RotateCcw size={14} />
                Reset to defaults
              </button>
            </div>

            {/* ── Content ─────────────────────────────── */}

            <div className="settings-content">
              <div className="settings-cat-header">
                <span className="settings-cat-icon">
                  {CategoryIcon && <CategoryIcon size={20} />}
                </span>
                <div>
                  <h3>{currentCategory?.label}</h3>
                  <p>{currentCategory?.description}</p>
                </div>
              </div>

              <div className="settings-fields">
                {settingsForCategory.map((meta) => {
                  const modified = isModified(meta.key, settings[meta.key as keyof typeof settings]);
                  return (
                    <div key={meta.key} className={`setting-row ${modified ? 'modified' : ''}`}>
                      <div className="setting-info">
                        <label className="setting-label" htmlFor={`setting-${meta.key}`}>
                          {meta.label}
                          {modified && (
                            <span className="setting-modified-dot" title="Modified from default">
                              <Dot size={12} />
                            </span>
                          )}
                        </label>
                        <span className="setting-desc">{meta.description}</span>
                      </div>
                      <div className="setting-control">
                        <SettingControl
                          value={settings[meta.key as keyof typeof settings]}
                          meta={meta}
                          onChange={(v) => updateSetting(meta.key as keyof typeof settings, v as never)}
                        />
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
