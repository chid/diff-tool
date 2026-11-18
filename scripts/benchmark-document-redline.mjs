// Node 22.18+. Optionally pass a path to an earlier redline.ts for a paired comparison.
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
const { compareDocuments } = await import(process.argv[2] ? pathToFileURL(resolve(process.argv[2])).href :
  new URL('../apps/desktop/src/documents/redline.ts', import.meta.url).href);
const runs = 7;
const paragraphs = Array.from({ length: 1500 }, (_, i) =>
  `Section ${i + 1}. The supplier shall provide the requested records within thirty days of receiving written notice.`);
const asDocument = text => ({ format: 'docx', sections: new Map([['Document body', text]]) });
const original = paragraphs.join('\n');
for (const [scenario, changed] of [
  ['One changed clause', i => i === 750],
  ['Every tenth clause changed', i => i % 10 === 0],
  ['Every clause changed', () => true],
]) {
  const revised = paragraphs.map((p, i) => changed(i) ? p.replace('thirty', 'sixty') : p).join('\n');
  const left = asDocument(original), right = asDocument(revised);
  const times = [];
  let retained;
  for (let i = 0; i <= runs; i++) {
    const start = performance.now();
    const [{ spans }] = compareDocuments(left, right);
    const elapsed = performance.now() - start;
    if (i) times.push(elapsed);
    assert.equal(spans.filter(s => !s.added).map(s => s.value).join(''), original);
    assert.equal(spans.filter(s => !s.removed).map(s => s.value).join(''), revised);
    retained = spans.reduce((n, s) => n + (!s.added && !s.removed ? s.value.length : 0), 0);
  }
  times.sort((a, b) => a - b);
  console.log(`${scenario}: ${times[Math.floor(runs / 2)].toFixed(1)} ms median, ${(retained * 100 / original.length).toFixed(2)}% of original characters retained`);
}
