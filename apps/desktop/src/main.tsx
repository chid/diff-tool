import React from 'react';
import ReactDOM from 'react-dom/client';
import { preloadHighlighter } from '@pierre/diffs';
import type { DiffsThemeNames, SupportedLanguages } from '@pierre/diffs';
import { App } from './App';
import './styles.css';

const LANGS: SupportedLanguages[] = ['tsx', 'typescript', 'rust', 'markdown'];
const THEMES: DiffsThemeNames[] = ['pierre-dark', 'pierre-light'];

function mountApp() {
  ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

// Preload the Shiki syntax highlighter so FileDiff can highlight synchronously
// on first paint. Fall back to rendering without highlighting if it fails.
preloadHighlighter({ langs: LANGS, themes: THEMES })
  .then(mountApp)
  .catch((err) => {
    console.warn('Highlighter preload failed, rendering without syntax highlighting:', err);
    mountApp();
  });
