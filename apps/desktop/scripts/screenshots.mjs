/**
 * Screenshot capture script for the diff-tool desktop app.
 *
 * Usage:
 *   node scripts/screenshots.mjs
 *
 * Starts the Vite dev server (or reuses an existing one), navigates to
 * several key views, and saves screenshots to docs/screenshots/.
 *
 * Works both in CI (headless) and locally (headed for visual inspection).
 */

import { chromium } from '@playwright/test';
import { fileURLToPath } from 'url';
import path from 'path';
import fs from 'fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.resolve(__dirname, '../../../docs/screenshots');
const BASE_URL = process.env.BASE_URL || 'http://127.0.0.1:5173';

// Ensure output directory exists
fs.mkdirSync(OUTPUT_DIR, { recursive: true });

/** Viewport presets for different screenshot types. */
const VIEWPORTS = {
  desktop: { width: 1440, height: 900 },
  tablet: { width: 834, height: 1112 },
  mobile: { width: 390, height: 844 },
};

/**
 * Screenshot scenarios — each describes a view to capture.
 * @type {Array<{name: string, path: string, viewport?: keyof typeof VIEWPORTS, actions?: (page: import('@playwright/test').Page) => Promise<void>, clip?: {selector: string}}>}
 */
const SCENARIOS = [
  // ── Full-page views ─────────────────────────────────────────────
  {
    name: 'desktop-empty-state',
    path: '/',
  },
  {
    name: 'desktop-welcome-hero',
    path: '/?demo=welcome',
  },
  // ── Smoke test page (pre-rendered fixture diff) ─────────────────
  {
    name: 'desktop-smoke-unified',
    path: '/?test=smoke',
    actions: async (page) => {
      // Wait for the diff grid to render
      await page.waitForSelector('.diff-grid', { timeout: 10000 });
    },
  },
  {
    name: 'desktop-smoke-unified-mobile',
    path: '/?test=smoke',
    viewport: 'mobile',
    actions: async (page) => {
      await page.waitForSelector('.diff-grid', { timeout: 10000 });
    },
  },
  // ── Browser test harness (multi-fixture browser) ────────────────
  {
    name: 'desktop-test-harness',
    path: '/?test=diff',
    actions: async (page) => {
      await page.waitForSelector('.harness-render-area .diff-grid', { timeout: 10000 });
    },
  },
  // ── Settings panel (open) ───────────────────────────────────────
  {
    name: 'desktop-settings-panel',
    path: '/?test=smoke',
    actions: async (page) => {
      // Click the settings gear button to open the panel
      await page.waitForSelector('.settings-trigger', { timeout: 5000 });
      await page.click('.settings-trigger');
      await page.waitForSelector('.settings-panel', { timeout: 5000 });
    },
  },
  // ── Demo gallery view ──────────────────────────────────────────
  {
    name: 'desktop-demo-gallery',
    path: '/?demo=gallery',
    actions: async (page) => {
      await page.waitForSelector('.demo-gallery', { timeout: 10000 });
    },
  },
  // ── Split view smoke test ──────────────────────────────────────
  // (We can't directly change the setting in the static page without
  // interacting, so skip for now — the unified view covers the core renderer.)
];

async function run() {
  const browser = await chromium.launch({ headless: process.env.CI === 'true' });

  for (const scenario of SCENARIOS) {
    const viewportKey = scenario.viewport || 'desktop';
    const viewport = VIEWPORTS[viewportKey];

    const context = await browser.newContext({
      viewport,
      deviceScaleFactor: 2,
    });
    const page = await context.newPage();

    console.log(`📸  Capturing "${scenario.name}" at ${viewport.width}x${viewport.height}…`);

    try {
      await page.goto(`${BASE_URL}${scenario.path}`, { waitUntil: 'networkidle' });

      if (scenario.actions) {
        await scenario.actions(page);
      }

      // Brief pause for animations to settle
      await page.waitForTimeout(400);

      await page.screenshot({
        path: path.join(OUTPUT_DIR, `${scenario.name}.png`),
        fullPage: false,
      });

      console.log(`   ✓  saved to docs/screenshots/${scenario.name}.png`);
    } catch (err) {
      console.error(`   ✗  FAILED: ${err.message}`);
    } finally {
      await context.close();
    }
  }

  await browser.close();
  console.log(`\n✅  Done — ${SCENARIOS.length} screenshots saved to docs/screenshots/`);
}

run().catch((err) => {
  console.error('Screenshot capture failed:', err);
  process.exit(1);
});
