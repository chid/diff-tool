import { governanceCases, type GovernanceCase, type GovernanceEdition } from './governance';

export const documentDemos = [
  { id: 'supplier', title: 'Supplier agreement', format: 'DOCX', description: 'Trace a payment deadline, a delivery day, and a clause that moved.' },
  { id: 'launch', title: 'Launch review', format: 'PPTX', description: 'Review a coffee launch with a larger first shipment and revised label artwork.' },
  { id: 'reading-room', title: 'Reading room floor plan', format: 'PDF', description: 'Find the aisle that opened between two tables while the labels stayed the same.' },
] as const;

type FileDemo = typeof documentDemos[number];
export type DocumentDemo = FileDemo | { id: string; format: 'TXT'; brief: GovernanceCase };

export const allDocumentDemos: DocumentDemo[] = [
  ...documentDemos,
  ...governanceCases.map(brief => ({ id: brief.id, format: 'TXT' as const, brief })),
];

export function demoFileUrl(demo: FileDemo, side: 'original' | 'revised') {
  return `${import.meta.env.BASE_URL}document-demos/${demo.id}-${side}.${demo.format.toLowerCase()}`;
}

export async function loadDemoPair(demo: DocumentDemo, signal: AbortSignal) {
  if (demo.format === 'TXT') {
    const excerptFile = (edition: GovernanceEdition) => new File(
      [edition.excerpt], `${demo.id}-${edition.date}-excerpt.txt`, { type: 'text/plain' },
    );
    return { left: excerptFile(demo.brief.before), right: excerptFile(demo.brief.after), demo };
  }
  const load = async (side: 'original' | 'revised') => {
    const url = demoFileUrl(demo, side);
    const response = await fetch(url, { signal });
    if (!response.ok) throw new Error(`Could not load the ${side} file (HTTP ${response.status}).`);
    return new File([await response.blob()], url.split('/').pop()!);
  };
  const [left, right] = await Promise.all([load('original'), load('revised')]);
  return { left, right, demo };
}
