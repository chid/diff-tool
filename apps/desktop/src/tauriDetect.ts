/** True when running inside a Tauri window (not plain Vite dev). */
export const isTauri: boolean =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
