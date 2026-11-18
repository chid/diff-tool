import { createWorker, OEM } from 'tesseract.js';
import workerUrl from 'tesseract.js/dist/worker.min.js?url';
import coreUrl from 'tesseract.js-core/tesseract-core-lstm.wasm.js?url';
import englishUrl from '@tesseract.js-data/eng/4.0.0_best_int/eng.traineddata.gz?url';

export async function englishOcr(signal?: AbortSignal) {
  signal?.throwIfAborted();
  const langPath = new URL('.', new URL(englishUrl, document.baseURI)).href;
  return createWorker('eng', OEM.LSTM_ONLY, {
    langPath,
    workerPath: workerUrl,
    corePath: coreUrl,
    cacheMethod: 'none',
  });
}
