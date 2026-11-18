import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  base: './',
  plugins: [react()],
  build: { rollupOptions: { output: {
    // Tesseract requests the model by language code relative to langPath.
    assetFileNames: asset => asset.name === 'eng.traineddata.gz'
      ? 'assets/ocr/eng.traineddata.gz' : 'assets/[name]-[hash][extname]',
  } } },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: false,
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
    css: true,
  },
});
