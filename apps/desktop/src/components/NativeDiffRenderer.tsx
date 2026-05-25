import type { DiffResult, LineChange, IntraSpan } from '../types';
import { useSettings } from './SettingsContext';

interface NativeDiffRendererProps {
  diff: DiffResult;
}

export function NativeDiffRenderer({ diff }: NativeDiffRendererProps) {
  const { settings } = useSettings();
  const isSplit = settings.diffStyle === 'split';

  if (diff.files.length === 0) {
    return (
      <article className="diff-pane diff-pane-center" aria-label="Empty diff">
        <p className="diff-status-text">No changes</p>
      </article>
    );
  }

  const file = diff.files[0];
  if (file.hunks.length === 0) {
    return (
      <article className="diff-pane diff-pane-center" aria-label="No changes">
        <p className="diff-status-text">Files are identical</p>
      </article>
    );
  }

  return (
    <article className="diff-pane" aria-label="Text diff">
      <div className="diff-grid">
        {isSplit ? (
          <SplitView file={file} />
        ) : (
          <UnifiedView file={file} />
        )}
      </div>
    </article>
  );
}

// ── Unified view ──────────────────────────────────────────────────────

function UnifiedView({ file }: { file: { left: string; right: string; hunks: import('../types').Hunk[] } }) {
  return (
    <>
      <div className="file-header">
        <span>{file.left} → {file.right}</span>
      </div>
      {file.hunks.map((hunk, hi) => (
        <div key={hi} className="hunk-group">
          {hunk.foldable_context && hi > 0 && (
            <div className="hunk-separator" aria-hidden="true">···</div>
          )}
          <div className="hunk-header">
            @@ -{hunk.old_range.start},{hunk.old_range.len} +{hunk.new_range.start},{hunk.new_range.len} @@
          </div>
          {hunk.lines.map((line, li) => (
            <UnifiedLine key={li} line={line} />
          ))}
        </div>
      ))}
    </>
  );
}

function UnifiedLine({ line }: { line: LineChange }) {
  const marker = line.kind === 'addition' ? '+' : line.kind === 'deletion' ? '−' : ' ';
  return (
    <div className={`diff-line ${line.kind}`}>
      <span className="line-no">{line.old_no ?? ''}</span>
      <span className="line-no">{line.new_no ?? ''}</span>
      <span className="marker">{marker}</span>
      <code>{renderIntraText(line)}</code>
    </div>
  );
}

// ── Split view ────────────────────────────────────────────────────────

function SplitView({ file }: { file: { left: string; right: string; hunks: import('../types').Hunk[] } }) {
  return (
    <>
      <div className="split-file-header">
        <div className="file-header">{file.left}</div>
        <div className="file-header">{file.right}</div>
      </div>
      {file.hunks.map((hunk, hi) => (
        <div key={hi} className="hunk-group-split">
          {hunk.foldable_context && hi > 0 && (
            <div className="hunk-separator split-separator" aria-hidden="true">···</div>
          )}
          <div className="hunk-header split-header">
            @@ -{hunk.old_range.start},{hunk.old_range.len} +{hunk.new_range.start},{hunk.new_range.len} @@
          </div>
          {hunk.lines.map((line, li) => (
            <SplitLine key={li} line={line} />
          ))}
        </div>
      ))}
    </>
  );
}

function SplitLine({ line }: { line: LineChange }) {
  const leftText = line.kind === 'addition' ? '' : line.text;
  const rightText = line.kind === 'deletion' ? '' : line.text;
  const leftNo = line.old_no ?? '';
  const rightNo = line.new_no ?? '';

  return (
    <div className={`diff-line-split ${line.kind}`}>
      <div className={`split-pane-col ${line.kind === 'deletion' ? 'deletion' : ''} ${line.kind === 'context' || line.kind === 'deletion' ? '' : 'empty-pane'}`}>
        <span className="line-no">{leftNo}</span>
        <code>{line.kind === 'deletion' ? renderIntraText(line) : leftText}</code>
      </div>
      <div className={`split-pane-col ${line.kind === 'addition' ? 'addition' : ''} ${line.kind === 'context' || line.kind === 'addition' ? '' : 'empty-pane'}`}>
        <span className="line-no">{rightNo}</span>
        <code>{line.kind === 'addition' ? renderIntraText(line) : rightText}</code>
      </div>
    </div>
  );
}

// ── Intraline span rendering ──────────────────────────────────────────

/**
 * Split a string into segments based on `intra_spans` so the UI can
 * highlight the specific characters that changed.
 *
 * Spans use byte offsets from the Rust backend (UTF-8). This function
 * converts them to JavaScript string indices (UTF-16 code-unit offsets)
 * so `String.prototype.slice` produces the correct substring.
 */
export function splitIntraText(text: string, spans: IntraSpan[]): { text: string; highlight: boolean }[] {
  if (spans.length === 0) {
    return [{ text, highlight: false }];
  }

  // Build a mapping: charIndex → cumulative UTF-8 byte offset.
  // charByteOffsets[k] = byte offset of the start of the k-th character.
  const charByteOffsets: number[] = [0];
  const encoder = new TextEncoder();
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    // Surrogate pairs (non-BMP) take 4 bytes in UTF-8; BMP takes 1-3.
    const code = ch.charCodeAt(0);
    let byteLen: number;
    if (code >= 0xd800 && code <= 0xdbff) {
      // High surrogate: the pair together encodes one 4-byte UTF-8 codepoint.
      byteLen = 4;
      i++; // skip the low surrogate in the loop
      // The low surrogate shares the same starting byte offset as the high surrogate.
      // Insert an entry so that JS slice indices for the surrogate pair are correct.
      charByteOffsets.push(charByteOffsets[charByteOffsets.length - 1]);
    } else {
      byteLen = encoder.encode(ch).length;
    }
    charByteOffsets.push(charByteOffsets[charByteOffsets.length - 1] + byteLen);
  }

  const segments: { text: string; highlight: boolean }[] = [];
  let cursor = 0; // character index

  for (const span of spans) {
    // Skip zero-length spans — they don't represent a change
    if (span.len === 0) continue;
    const byteStart = span.start;
    const byteEnd = byteStart + span.len;

    // Find the character index where the cumulative byte offset reaches byteStart.
    let charStart = cursor;
    while (charStart < charByteOffsets.length && charByteOffsets[charStart] < byteStart) {
      charStart++;
    }
    // Find the character index where the cumulative byte offset reaches byteEnd.
    let charEnd = charStart;
    while (charEnd < charByteOffsets.length && charByteOffsets[charEnd] < byteEnd) {
      charEnd++;
    }

    // Clamp to valid range
    if (charStart > text.length || charStart < cursor) continue;
    charEnd = Math.min(charEnd, text.length);

    // Text before the span
    if (charStart > cursor) {
      segments.push({ text: text.slice(cursor, charStart), highlight: false });
    }

    // The span itself
    if (charEnd > charStart) {
      segments.push({ text: text.slice(charStart, charEnd), highlight: true });
    }

    cursor = charEnd;
  }

  // Remaining text after the last span
  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), highlight: false });
  }

  return segments.length > 0 ? segments : [{ text, highlight: false }];
}

/** Render a line's text with intraline highlights. */
function renderIntraText(line: LineChange): React.ReactNode {
  if (line.intra_spans.length === 0) {
    return line.text;
  }

  const segments = splitIntraText(line.text, line.intra_spans);
  return (
    <>
      {segments.map((seg, i) =>
        seg.highlight ? (
          <span key={i} className="intra-highlight">{seg.text}</span>
        ) : (
          <span key={i}>{seg.text}</span>
        )
      )}
    </>
  );
}
