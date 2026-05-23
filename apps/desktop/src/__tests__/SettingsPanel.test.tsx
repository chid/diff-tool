import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { SettingsProvider } from '../components/SettingsContext';
import { SettingsPanel } from '../components/SettingsPanel';
import { SETTINGS_DEFAULTS, CATEGORIES, SETTINGS_REGISTRY, settingsToFileDiffOptions } from '../settingsTypes';
import type { SettingsState } from '../settingsTypes';

// ─── Helpers ───────────────────────────────────────────────────────────────

const STORAGE_KEY = 'diff-tool-settings';

function mockStorage(initial?: Partial<SettingsState>) {
  const store: Record<string, string> = {};
  if (initial) store[STORAGE_KEY] = JSON.stringify({ ...SETTINGS_DEFAULTS, ...initial });
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

function renderPanel(initial?: Partial<SettingsState>) {
  mockStorage(initial);
  return render(
    <SettingsProvider>
      <SettingsPanel />
    </SettingsProvider>,
  );
}

function openPanel() {
  const trigger = screen.getByRole('button', { name: /settings/i });
  fireEvent.click(trigger);
}

/** Get the sidebar <nav> within the dialog */
function getSidebarNav() {
  const dialog = screen.getByRole('dialog');
  return within(dialog).getByRole('navigation', { name: 'Settings categories' });
}

/** Click a category button by label text */
function getCategoryButton(label: string) {
  const nav = getSidebarNav();
  const el = within(nav).getByText(label).closest('button')!;
  return el;
}

/** Get the settings panel div for focus-trap queries */
function getPanelElement() {
  const dialog = screen.getByRole('dialog');
  return dialog.querySelector('.settings-panel')!;
}

// ─── Tests ─────────────────────────────────────────────────────────────────

describe('SettingsPanel', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  describe('open / close', () => {
    it('renders a settings trigger button', () => {
      renderPanel();
      expect(screen.getByRole('button', { name: /settings/i })).toBeInTheDocument();
    });

    it('opens the panel when trigger is clicked', () => {
      renderPanel();
      openPanel();
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    it('closes on Escape key (with animation)', async () => {
      renderPanel();
      openPanel();
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      fireEvent.keyDown(window, { key: 'Escape' });

      // Close triggers a 180ms CSS animation + React state update
      await waitFor(
        () => {
          expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
        },
        { timeout: 500 },
      );
    });

    it('closes on overlay click (with animation)', async () => {
      renderPanel();
      openPanel();

      const overlay = screen.getByRole('dialog');
      fireEvent.click(overlay);

      await waitFor(
        () => {
          expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
        },
        { timeout: 500 },
      );
    });

    it('does not close when clicking inside the panel', () => {
      renderPanel();
      openPanel();

      // Click on a category button inside the panel
      const perfBtn = getCategoryButton('Performance');
      fireEvent.click(perfBtn);

      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    it('shows modified count badge on trigger when settings differ', () => {
      renderPanel({ diffStyle: 'unified', disableLineNumbers: true });
      const trigger = screen.getByRole('button', { name: /settings/i });
      expect(trigger.textContent).toContain('2');
    });
  });

  describe('category navigation', () => {
    it('renders all category tabs with icons', () => {
      renderPanel();
      openPanel();

      for (const cat of CATEGORIES) {
        expect(getCategoryButton(cat.label)).toBeInTheDocument();
      }
    });

    it('starts on the Display category', () => {
      renderPanel();
      openPanel();

      expect(getCategoryButton('Display')).toHaveClass('active');
    });

    it('switches to a different category on click', () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Performance'));

      expect(getCategoryButton('Performance')).toHaveClass('active');
      expect(getCategoryButton('Display')).not.toHaveClass('active');
    });

    it('shows category header with icon and description', () => {
      renderPanel();
      openPanel();

      const dialog = screen.getByRole('dialog');
      // Scope to the content area only (sidebar nav also has the description)
      const content = dialog.querySelector('.settings-content')!;
      expect(within(content as HTMLElement).getByText('Visual layout and appearance')).toBeInTheDocument();

      fireEvent.click(getCategoryButton('Performance'));
      expect(within(content as HTMLElement).getByText('Worker threads and rendering budgets')).toBeInTheDocument();
    });

    it('shows settings for the active category', () => {
      renderPanel();
      openPanel();

      // Display category includes diffStyle
      const dialog = screen.getByRole('dialog');
      expect(within(dialog).getByText('Diff style')).toBeInTheDocument();

      // Switch to Performance
      fireEvent.click(getCategoryButton('Performance'));

      // Should show the Web Workers setting
      expect(within(dialog).getByText('Disable Web Workers')).toBeInTheDocument();
    });

    it('shows modified badge per category', () => {
      renderPanel({ disableWorkerPool: false, diffStyle: 'unified' });
      openPanel();

      // Performance category should show badge (disableWorkerPool modified)
      const perfBtn = getCategoryButton('Performance');
      expect(perfBtn.textContent).toContain('1');

      // File Display should not have badge
      const fileBtn = getCategoryButton('File Display');
      expect(fileBtn.querySelector('.settings-nav-badge')).toBeNull();
    });
  });

  describe('focus trap', () => {
    it('has focusable elements in the panel', () => {
      renderPanel();
      openPanel();

      const panel = getPanelElement();
      const focusable = panel.querySelectorAll<HTMLElement>(
        'button, input, select, textarea',
      );

      expect(focusable.length).toBeGreaterThan(0);
    });

    it('panel has correct dialog ARIA attributes', () => {
      renderPanel();
      openPanel();

      const dialog = screen.getByRole('dialog');
      expect(dialog).toHaveAttribute('aria-modal', 'true');
      expect(dialog).toHaveAttribute('aria-label', 'Settings');
    });
  });

  describe('toggle settings', () => {
    it('renders toggle for boolean settings', () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Interaction'));

      const toggle = screen.getByRole('switch', { name: 'Sticky header' });
      expect(toggle).toBeInTheDocument();
      expect(toggle).toHaveAttribute('aria-checked', 'true');
    });

    it('toggles a boolean setting on click', () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Interaction'));

      const toggle = screen.getByRole('switch', { name: 'Sticky header' });
      expect(toggle).toHaveAttribute('aria-checked', 'true');

      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute('aria-checked', 'false');

      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute('aria-checked', 'true');
    });

    it('toggles the Web Workers setting', () => {
      renderPanel({ disableWorkerPool: true });
      openPanel();

      fireEvent.click(getCategoryButton('Performance'));

      const toggle = screen.getByRole('switch', { name: 'Disable Web Workers' });
      expect(toggle).toHaveAttribute('aria-checked', 'true');

      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute('aria-checked', 'false');
    });
  });

  describe('select settings', () => {
    it('renders select for string options', () => {
      renderPanel();
      openPanel();

      const select = screen.getByLabelText('Diff style') as HTMLSelectElement;
      expect(select.tagName).toBe('SELECT');
      expect(select.value).toBe('split');
    });

    it('changes a select value', () => {
      renderPanel();
      openPanel();

      const select = screen.getByLabelText('Diff style') as HTMLSelectElement;
      fireEvent.change(select, { target: { value: 'unified' } });

      expect(select.value).toBe('unified');
    });

    it('changes theme select', () => {
      renderPanel();
      openPanel();

      const select = screen.getByLabelText('Theme') as HTMLSelectElement;
      fireEvent.change(select, { target: { value: 'pierre-light' } });

      expect(select.value).toBe('pierre-light');
    });

    it('changes lineDiffType select', () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Diff Algorithm'));

      const select = screen.getByLabelText('Inline diff');
      fireEvent.change(select, { target: { value: 'word' } });

      expect((select as HTMLSelectElement).value).toBe('word');
    });
  });

  describe('number settings', () => {
    it('renders number input for numeric settings', () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Diff Algorithm'));

      const input = screen.getByLabelText('Context threshold') as HTMLInputElement;
      expect(input.type).toBe('number');
      expect(Number(input.value)).toBe(SETTINGS_DEFAULTS.collapsedContextThreshold);
    });

    it('changes a number value on blur', async () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Diff Algorithm'));

      const input = screen.getByLabelText('Context threshold') as HTMLInputElement;
      fireEvent.change(input, { target: { value: '10' } });
      fireEvent.blur(input);

      await waitFor(() => {
        expect(Number(input.value)).toBe(10);
      });
    });

    it('clamps number to min/max', async () => {
      renderPanel();
      openPanel();

      fireEvent.click(getCategoryButton('Diff Algorithm'));

      const input = screen.getByLabelText('Context threshold') as HTMLInputElement;
      fireEvent.change(input, { target: { value: '999' } });
      fireEvent.blur(input);

      await waitFor(() => {
        expect(Number(input.value)).toBe(25); // max is 25
      });
    });
  });

  describe('reset', () => {
    it('renders a reset button', () => {
      renderPanel();
      openPanel();

      expect(screen.getByText('Reset to defaults')).toBeInTheDocument();
    });

    it('resets settings on confirm', () => {
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
      renderPanel({ diffStyle: 'unified' });
      openPanel();

      fireEvent.click(screen.getByText('Reset to defaults'));

      const select = screen.getByLabelText('Diff style') as HTMLSelectElement;
      expect(select.value).toBe(SETTINGS_DEFAULTS.diffStyle);

      confirmSpy.mockRestore();
    });

    it('does not reset if user cancels', () => {
      const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
      renderPanel({ diffStyle: 'unified' });
      openPanel();

      fireEvent.click(screen.getByText('Reset to defaults'));

      const select = screen.getByLabelText('Diff style') as HTMLSelectElement;
      expect(select.value).toBe('unified');

      confirmSpy.mockRestore();
    });
  });

  describe('modified indicators', () => {
    it('shows modified dot next to changed settings', () => {
      renderPanel({ diffStyle: 'unified' });
      openPanel();

      // The diffStyle row should have the "modified" class
      const rows = screen.getByRole('dialog').querySelectorAll('.setting-row');
      const modifiedRows = screen.getByRole('dialog').querySelectorAll('.setting-row.modified');
      expect(modifiedRows.length).toBeGreaterThan(0);
    });
  });

  describe('JSON-driven: all settings exist', () => {
    it('renders every registered setting across all categories', () => {
      renderPanel();
      openPanel();

      for (const cat of CATEGORIES) {
        fireEvent.click(getCategoryButton(cat.label));

        const dialog = screen.getByRole('dialog');
        for (const meta of SETTINGS_REGISTRY.filter((m) => m.category === cat.key)) {
          const el = within(dialog).getByText(meta.label);
          expect(el).toBeInTheDocument();
        }
      }
    });

    it('every JSON setting has a matching control in the DOM', () => {
      renderPanel();
      openPanel();

      // All 25 settings should have a row
      let totalRows = 0;
      for (const cat of CATEGORIES) {
        fireEvent.click(getCategoryButton(cat.label));
        const dialog = screen.getByRole('dialog');
        const rows = dialog.querySelectorAll('.setting-row');
        totalRows += rows.length;
      }
      expect(totalRows).toBe(SETTINGS_REGISTRY.length);
    });
  });

  describe('settingsToFileDiffOptions integration', () => {
    it('maps interaction settings into output', () => {
      const defaults = SETTINGS_DEFAULTS;
      const result = settingsToFileDiffOptions(defaults);

      expect(result).toHaveProperty('lineHoverHighlight', defaults.lineHoverHighlight);
      expect(result).toHaveProperty('enableLineSelection', defaults.enableLineSelection);
      expect(result).toHaveProperty(
        'enableTokenInteractionsOnWhitespace',
        defaults.enableTokenInteractionsOnWhitespace,
      );
    });

    it('JSON registry covers all settingsToFileDiffOptions keys', () => {
      const defaults = SETTINGS_DEFAULTS;
      const result = settingsToFileDiffOptions(defaults);
      const registryKeys = new Set(SETTINGS_REGISTRY.map((m) => m.key));

      for (const key of Object.keys(result)) {
        expect(registryKeys.has(key)).toBe(true);
      }
    });
  });
});
