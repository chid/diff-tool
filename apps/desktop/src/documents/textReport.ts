import type { Section, Span } from './redline';
import type { ReviewReport } from './reviewReport';

type ReportItem = ReviewReport['items'][number];
const changed = (span: Span) => span.added || span.removed;

// Bound context, not the edited passage. The detailed report keeps every changed word.
function surrounding(left: string, right: string) {
  const tail = left.slice(-120);
  const head = right.slice(0, 120);
  let prefix = tail.split(/\n|(?<=[.!?])\s+/u).at(-1) ?? '';
  let suffix = head.match(/^[^\n]*?[.!?](?=\s|$)|^[^\n]*/u)?.[0] ?? '';
  if (prefix === tail && left.length > tail.length) prefix = '…' + prefix.replace(/^\S*\s/u, ' ');
  if (suffix === head && right.length > head.length) suffix = suffix.replace(/\s\S*$/u, ' ') + '…';
  return { prefix, suffix };
}

function shortQuote(text: string, limit: number) {
  const flat = text.trim().replace(/\s+/gu, ' ');
  if (flat.length <= limit) return '“' + flat + '”';
  const end = Math.floor((limit - 3) / 2);
  return '“' + flat.slice(0, limit - end - 3).trimEnd() + ' … ' + flat.slice(-end).trimStart() + '”';
}

function visibleWhitespace(value: string) {
  return value.replace(/ /g, '·').replace(/\t/g, '⇥').replace(/\r/g, '␍').replace(/\n/g, '↵\n');
}

function textEdits(section: Section): ReportItem[] {
  const items: ReportItem[] = [];
  const movedText = new Set(section.moves?.map(move => move.text.trim()));
  const spans = section.spans;
  for (let i = 0; i < spans.length;) {
    if (!changed(spans[i])) { i++; continue; }
    const start = i;
    let oldWords = '', newWords = '';
    while (i < spans.length) {
      const span = spans[i];
      if (!changed(span)) {
        // Closely spaced edits in the same sentence belong in a single pair of quotations.
        // Build each side from its own spans, so neither contains words from the other version.
        if (!spans[i + 1] || !changed(spans[i + 1]) || span.value.length > 80 || /[\r\n]|[.!?](?:\s|$)/u.test(span.value) ||
            /[\r\n]/u.test(spans[i - 1].value) || /[\r\n]/u.test(spans[i + 1].value)) break;
        oldWords += span.value;
        newWords += span.value;
      }
      if (span.added) newWords += span.value;
      if (span.removed) oldWords += span.value;
      i++;
    }
    const oldTrimmed = oldWords.trim(), newTrimmed = newWords.trim();
    if ((movedText.has(oldTrimmed) || movedText.has(newTrimmed)) &&
        (!oldTrimmed || movedText.has(oldTrimmed)) && (!newTrimmed || movedText.has(newTrimmed))) continue;

    const { prefix, suffix } = surrounding(spans[start - 1]?.value ?? '', spans[i]?.value ?? '');
    if (oldWords.replace(/\s/gu, '') === newWords.replace(/\s/gu, '')) {
      items.push({
        title: section.label + ' · Spacing changed',
        description: 'The edited whitespace is marked with · for spaces, ⇥ for tabs, ␍ for carriage returns and ↵ for line breaks.',
        before: prefix + visibleWhitespace(oldWords) + suffix, after: prefix + visibleWhitespace(newWords) + suffix,
      });
      continue;
    }
    // Keep a following unit with numeric edits when it is common to both readings.
    const unit = /^\d[\d.,]*$/u.test(oldTrimmed) && /^\d[\d.,]*$/u.test(newTrimmed)
      ? suffix.match(/^\s+(?:(?:calendar|working|business)\s+)?(?:days?|weeks?|months?|years?|hours?|minutes?|seconds?)\b/iu)?.[0] ?? '' : '';
    const description = oldTrimmed && newTrimmed
      ? 'Changed ' + shortQuote(oldTrimmed + unit, 75) + ' to ' + shortQuote(newTrimmed + unit, 75) + '.'
      : (newTrimmed ? 'Added ' + shortQuote(newWords, 100) : 'Removed ' + shortQuote(oldWords, 100)) + '.';
    items.push({
      title: section.label,
      description,
      before: (prefix + oldWords + suffix).trim() || undefined,
      after: (prefix + newWords + suffix).trim() || undefined,
    });
  }

  for (const move of section.moves ?? []) {
    items.push({
      title: section.label + ' · Moved paragraph',
      description: `Paragraph ${move.from} moved to paragraph ${move.to} with its wording intact.`,
      after: move.text,
    });
  }
  return items;
}

function describeSection(section: Section): ReportItem[] {
  if (/^(?:word|ppt|xl)\/media\//.test(section.label)) {
    const description = section.status === 'modified'
      ? 'The embedded file at this package path has different bytes in the two documents.'
      : `The embedded file was ${section.status}.`;
    return [{
      title: section.label, description,
      before: section.spans.find(span => span.removed)?.value,
      after: section.spans.find(span => span.added)?.value,
    }];
  }
  if (/^Table \d+ row \d+ structure$/.test(section.label)) {
    const before = section.spans.filter(span => !span.added).map(span => span.value).join('');
    const after = section.spans.filter(span => !span.removed).map(span => span.value).join('');
    const cellCount = (value: string) => value ? value.split(' | ').length : 0;
    const cells = (count: number) => count + (count === 1 ? ' cell' : ' cells');
    const previous = cellCount(before), current = cellCount(after);
    const description = section.status === 'added' ? 'Added a row with ' + cells(current) + '.'
      : section.status === 'removed' ? 'Removed a row with ' + cells(previous) + '.'
      : previous === current ? 'The row contains ' + cells(current) + ' and its cell merge settings changed.'
      : 'The row changed from ' + cells(previous) + ' to ' + cells(current) + '.';
    return [{ title: section.label, description, before: before || undefined, after: after || undefined }];
  }
  if (section.status === 'modified') return textEdits(section);
  const content = section.spans.map(span => span.value).join('');
  return [{
    title: section.label, description: `This section was ${section.status}.` +
      (content.trim() ? '' : ' It contains no visible text.'),
    ...(section.status === 'added' ? { after: content } : { before: content }),
  }];
}

function highlight(item: ReportItem) {
  const location = item.title === 'Document body' || item.title.startsWith('Document body ·') ? '' : item.title + ' — ';
  if (item.before && item.after && item.description?.startsWith('Changed ')) {
    return location + item.description.slice(0, -1) + ' in ' + shortQuote(item.after, 175);
  }
  const context = item.after || item.before;
  return location + item.description + (context ? ' ' + shortQuote(context, 120) : '');
}

export function textReport(sections: Section[], notes: string[], original: string, revised: string): ReviewReport {
  const pdf = [original, revised].some(name => /\.pdf$/i.test(name));
  const changedSections = sections.filter(section => section.status !== 'unchanged');
  const items: ReportItem[] = [], highlights: string[] = [], extra: ReportItem[] = [];
  for (const section of changedSections) {
    const entries = describeSection(section);
    for (const entry of entries) items.push(entry);
    if (entries.length && highlights.length < 4) highlights.push(highlight(entries[0]));
    for (let i = 1; i < entries.length && extra.length < 3; i++) extra.push(entries[i]);
  }
  for (const entry of extra.slice(0, 4 - highlights.length)) highlights.push(highlight(entry));
  const count = changedSections.length;
  const scope = [...notes];
  if (pdf) scope.push('PDF text comparison uses extracted body text and normalizes whitespace. Drawings and page layout require visual review.');
  return {
    mode: pdf ? 'PDF text' : 'Document text and structure',
    summary: !sections.length ? 'There were no comparable sections.' : count
      ? `${count} ${count === 1 ? 'section changed' : 'sections changed'} in the compared content.`
      : 'No differences were found in the compared content.',
    highlights,
    notes: scope,
    items,
  };
}
