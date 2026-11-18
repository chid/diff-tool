import { governanceCases, type GovernanceCase } from './governance';

const monthYear = new Intl.DateTimeFormat('en', { month: 'short', year: 'numeric', timeZone: 'UTC' });
const editionMonth = (date: string) => monthYear.format(new Date(date));

export function GovernanceExamples() {
  return <section className="governance-examples" aria-label="AI governance reviews">
    <h2>AI governance reviews</h2>
    <p>Compare two published editions, inspect the sources, and take a specific question into a client review.</p>
    <div className="governance-cards">
      {governanceCases.map(brief => <a key={brief.id} href={`?tab=documents&example=${brief.id}`}
        aria-label={`${brief.company} governance review`}>
        <strong>{brief.company}</strong>
        <span>{brief.title}</span>
        <small>{editionMonth(brief.before.date)} – {editionMonth(brief.after.date)} · {brief.document}</small>
      </a>)}
    </div>
  </section>;
}

export function GovernanceBrief({ brief, onDownload }: { brief: GovernanceCase; onDownload: () => void }) {
  return <section className="governance-brief" aria-label="Consultant brief">
    <span className="governance-document">{brief.document}</span>
    <h2>{brief.title}</h2>
    <p>{brief.scope}</p>
    <button type="button" onClick={onDownload}>Download review report</button>
    <h3>What changed</h3>
    <p>{brief.finding}</p>
    <div className="governance-notes">
      {([['Earlier', brief.before], ['Later', brief.after]] as const).map(([label, edition]) =>
        <div key={label}>
          <h3>{label} edition</h3>
          <blockquote cite={edition.url}>{edition.excerpt}</blockquote>
          <p><strong>Analyst note.</strong> {edition.text}</p>
          <a href={edition.url} target="_blank" rel="noreferrer">{edition.label} · <time dateTime={edition.date}>{edition.date}</time></a>
        </div>)}
    </div>
    <h3>Client follow-up</h3>
    <p>{brief.question}</p>
    {brief.reference && <p><a href={brief.reference.url} target="_blank" rel="noreferrer">{brief.reference.label}</a></p>}
  </section>;
}
