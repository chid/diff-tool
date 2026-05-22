import type { DiffResult, LineChange } from '../types';

interface DiffPaneProps {
  diff: DiffResult;
}

export function DiffPane({ diff }: DiffPaneProps) {
  const file = diff.files[0];
  return (
    <article className="diff-pane" aria-label="Text diff">
      <header className="file-header">
        <span>{file.left}</span>
        {diff.truncated ? <strong>Truncated</strong> : null}
      </header>
      <div className="diff-grid" role="table" aria-label="Split diff">
        {file.hunks.flatMap((hunk, hunkIndex) =>
          hunk.lines.map((line, lineIndex) => (
            <DiffLine key={`${hunkIndex}-${lineIndex}`} line={line} />
          )),
        )}
      </div>
    </article>
  );
}

function DiffLine({ line }: { line: LineChange }) {
  const marker = line.kind === 'addition' ? '+' : line.kind === 'deletion' ? '-' : ' ';
  return (
    <div className={`diff-line ${line.kind}`} role="row">
      <span className="line-no">{line.old_no ?? ''}</span>
      <span className="line-no">{line.new_no ?? ''}</span>
      <span className="marker">{marker}</span>
      <code>{line.text || ' '}</code>
    </div>
  );
}
