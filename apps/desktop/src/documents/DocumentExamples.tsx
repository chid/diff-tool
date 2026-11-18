import { FileText, Presentation, Map as MapIcon } from 'lucide-react';
import { documentDemos } from './demos';

const icons = { DOCX: FileText, PPTX: Presentation, PDF: MapIcon };

export function DocumentExamples({ compact }: { compact: boolean }) {
  const links = <div className="document-example-grid">
    {documentDemos.map(demo => {
      const Icon = icons[demo.format];
      return <a key={demo.id} href={`?tab=documents&example=${demo.id}`} className="document-example" aria-label={demo.title}>
        <img className="document-example-preview" src={`${import.meta.env.BASE_URL}document-demos/${demo.id}-preview.webp`} alt="" loading="lazy" width="600" height="320" />
        <span className="document-example-format"><Icon size={17} aria-hidden="true" /> {demo.format}</span>
        <strong>{demo.title}</strong>
        <span>{demo.description}</span>
      </a>;
    })}
  </div>;
  return compact
    ? <details className="document-examples"><summary>Try another example</summary>{links}</details>
    : <section className="document-examples" aria-label="Document examples"><h2>Try a comparison</h2>{links}</section>;
}
