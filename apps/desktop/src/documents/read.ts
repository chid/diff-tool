import { strFromU8, unzipSync } from 'fflate';

export type Format = 'docx' | 'pdf' | 'xlsx' | 'pptx' | 'txt';
export interface ParsedDocument {
  format: Format;
  ocrPages?: number[];
  media?: Map<string, Uint8Array>;
  // Order is meaningful for display. Keys match sections across versions.
  sections: Map<string, string>;
}
export const accepted = '.docx,.pdf,.xlsx,.pptx,.txt';

const MiB = 1024 * 1024;
const body = 'Document body';
const elements = (node: Document | Element, name: string) =>
  Array.from(node.getElementsByTagNameNS('*', name));
const textRuns = (node: Element) => elements(node, 't').map(t => t.textContent).join('');

function loadPackage(bytes: Uint8Array) {
  let total = 0;
  let entries = 0;
  const parts = unzipSync(bytes, { filter(entry) {
    if (++entries > 12000) throw new Error('This package contains too many entries to compare.');
    const xmlPart = /^(word|xl|ppt)\/.*\.(xml|rels)$/.test(entry.name);
    const mediaPart = /^(word|xl|ppt)\/media\/[^/]+$/.test(entry.name);
    if (!xmlPart && !mediaPart) return false;
    total += entry.originalSize;
    if (entry.originalSize > 16 * MiB || total > 64 * MiB) throw new Error('The unpacked document exceeds the comparison size limit.');
    return true;
  } });
  function xml(path: string) {
    if (!Object.hasOwn(parts, path)) throw new Error(`Missing document part: ${path}`);
    const doc = new DOMParser().parseFromString(strFromU8(parts[path]), 'application/xml');
    if (elements(doc, 'parsererror').length) throw new Error(`Unreadable XML in ${path}`);
    return doc;
  }
  function related(path: string) {
    const name = path.split('/').pop();
    const rels = xml(path.replace(/[^/]+$/, `_rels/${name}.rels`));
    const targets = new Map(elements(rels, 'Relationship').filter(r => r.getAttribute('TargetMode') !== 'External').map(r => [
      r.getAttribute('Id'), new URL(r.getAttribute('Target') ?? '', 'https://package.invalid/' + path).pathname.slice(1),
    ]));
    return (ref: Element) => {
      const id = Array.from(ref.attributes).find(a => a.localName === 'id' && a.namespaceURI?.endsWith('/relationships'));
      const target = targets.get(id?.value ?? '');
      if (!target) throw new Error(`Missing relationship in ${path}`);
      return xml(target);
    };
  }
  const media = new Map(Object.entries(parts).filter(([path]) => path.includes('/media/')));
  return { xml, related, names: Object.keys(parts), media };
}
type Package = ReturnType<typeof loadPackage>;

function wordText(node: Element): string {
  // Walk once so paragraphs inside tables or text boxes are included once.
  const pieces: string[] = [];
  function walk(el: Element) {
    if (!el.namespaceURI?.endsWith('/wordprocessingml/main') &&
        !el.namespaceURI?.endsWith('/wordprocessingml/2006/main')) {
      for (const child of el.children) walk(child);
      return;
    }
    switch (el.localName) {
      case 'del': case 'moveFrom': return;
      case 't': pieces.push(el.textContent ?? ''); return;
      case 'tab': pieces.push('\t'); return;
      case 'br': case 'cr': pieces.push('\n'); return;
    }
    for (const child of el.children) walk(child);
    if (el.localName === 'p') pieces.push('\n');
  }
  walk(node);
  return pieces.join('').replace(/\n$/, '');
}

function wordFormatting(root: Element) {
  function describe(el: Element): string {
    const attrs = Array.from(el.attributes)
      .filter(a => !a.name.startsWith('xmlns'))
      .map(a => `${a.localName}=${a.value}`).sort().join(' ');
    const children = Array.from(el.children).filter(c => !c.localName.endsWith('Change')).map(describe).sort();
    return [el.localName, attrs, ...children].filter(Boolean).join(' ');
  }
  const formatted: string[] = [];
  for (const p of elements(root, 'p')) {
    if (p.closest('*|del, *|moveFrom')) continue;
    const props = Array.from(p.children).find(c => c.localName === 'pPr');
    if (props?.childElementCount) formatted.push(`Paragraph "${wordText(p)}" · ${describe(props)}`);
    let last = '', text = '';
    const flush = () => { if (last && text) formatted.push(`"${text}" · ${last}`); };
    for (const run of elements(p, 'r')) {
      if (run.closest('*|p') !== p || run.closest('*|del, *|moveFrom')) continue;
      const rPr = Array.from(run.children).find(c => c.localName === 'rPr');
      const style = rPr?.childElementCount ? describe(rPr) : '';
      if (style !== last) { flush(); last = style; text = ''; }
      text += wordText(run);
    }
    flush();
  }
  return formatted.join('\n');
}

function word(pkg: Package, sections: Map<string, string>) {
  const doc = pkg.xml('word/document.xml');
  const main = elements(doc, 'body')[0];
  if (!main) throw new Error('The Word file has no document body.');
  sections.set(body, wordText(main));
  const formatting = wordFormatting(main);
  if (formatting) sections.set('Word body formatting', formatting);
  for (const [i, table] of elements(main, 'tbl').entries()) {
    const rows = elements(table, 'tr').filter(r => r.closest('*|tbl') === table);
    for (const [r, row] of rows.entries()) {
      const cells = elements(row, 'tc').filter(c => c.closest('*|tr') === row);
      const structure = cells.map(c => {
        const props = Array.from(c.children).find(el => el.localName === 'tcPr');
        const values = props ? Array.from(props.children).filter(el => ['gridSpan', 'vMerge', 'hMerge'].includes(el.localName)) : [];
        return values.map(el => el.localName + '=' + (Array.from(el.attributes).find(a => a.localName === 'val')?.value ?? 'continue')).join(',') || 'cell';
      });
      sections.set(`Table ${i + 1} row ${r + 1} structure`, structure.join(' | '));
    }
  }
  for (const path of pkg.names.sort()) {
    const match = /^word\/(header\d+|footer\d+|footnotes|endnotes|comments)\.xml$/.exec(path);
    if (match) sections.set(match[1], wordText(pkg.xml(path).documentElement));
  }
}

function excel(pkg: Package, sections: Map<string, string>) {
  const path = 'xl/workbook.xml';
  const workbook = pkg.xml(path);
  if (workbook.documentElement.localName !== 'workbook') throw new Error('Invalid Excel workbook.');
  const sheets = elements(workbook, 'sheet');
  const getSheet = pkg.related(path);
  const shared = pkg.names.includes('xl/sharedStrings.xml')
    ? elements(pkg.xml('xl/sharedStrings.xml'), 'si').map(textRuns) : [];
  sections.set('Sheet order', sheets.map(s => s.getAttribute('name')).join('\n'));
  for (const sheet of sheets) {
    const sheetName = sheet.getAttribute('name') ?? '';
    for (const cell of elements(getSheet(sheet), 'c')) {
      const ref = cell.getAttribute('r');
      if (!ref) throw new Error(`A cell in ${sheetName} has no address.`);
      const type = cell.getAttribute('t');
      const formula = elements(cell, 'f')[0];
      const value = elements(cell, 'v')[0]?.textContent;
      if (!formula && value == null && type !== 'inlineStr') continue;
      let content = value ?? '';
      if (type === 'inlineStr') content = textRuns(cell);
      if (type === 's') {
        if (!/^\d+$/.test(content) || shared[Number(content)] === undefined) throw new Error(`Invalid shared string at ${sheetName}!${ref}`);
        content = shared[Number(content)];
      }
      const key = `${sheetName}!${ref}`;
      sections.set(key, content);
      const valueType = { s: 'text', inlineStr: 'text', str: 'text', b: 'boolean', d: 'date', e: 'error' }[type ?? 'n'];
      if (valueType) sections.set(`${key} type`, valueType);
      if (formula) {
        const attrs = Array.from(formula.attributes).map(a => `${a.name}=${a.value}`).sort();
        sections.set(`${key} formula`, ['=' + formula.textContent, ...attrs].join(' '));
      }
    }
  }
}

function powerpoint(pkg: Package, sections: Map<string, string>) {
  const path = 'ppt/presentation.xml';
  const deck = pkg.xml(path);
  if (deck.documentElement.localName !== 'presentation') throw new Error('Invalid PowerPoint presentation.');
  const slides = elements(deck, 'sldId');
  const getSlide = pkg.related(path);
  for (const [index, slide] of slides.entries()) {
    sections.set(`Slide ${index + 1}`, elements(getSlide(slide), 'p').map(textRuns).join('\n'));
  }
}

export async function readDocument(file: File, signal?: AbortSignal): Promise<ParsedDocument> {
  if (file.size > 50 * MiB) throw new Error('The comparison limit is 50 MiB per file.');
  const format = file.name.split('.').pop()?.toLowerCase();
  const sections = new Map<string, string>();
  switch (format) {
    case 'txt':
      sections.set(body, (await file.text()).replace(/\r\n?/g, '\n'));
      break;
    case 'pdf': {
      const { pdfText } = await import('./pdf');
      const pdf = await pdfText(file, signal);
      sections.set(body, pdf.text);
      return { format, sections, ocrPages: pdf.recognized };
    }
    case 'docx': case 'xlsx': case 'pptx': {
      const pkg = loadPackage(new Uint8Array(await file.arrayBuffer()));
      ({ docx: word, xlsx: excel, pptx: powerpoint })[format](pkg, sections);
      return { format, sections, media: pkg.media };
    }
    default: throw new Error(`Choose a supported file (${accepted}).`);
  }
  return { format, sections };
}
