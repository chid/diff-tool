# Browser document comparison

Open **Documents**, choose an original and a revision, or drop two files on the app in that order. The direct route is `?tab=documents`. Document contents, PDF rendering, and English OCR stay in the browser. The app serves the OCR worker, engine, and language model from its own assets on demand.

## Coverage against Litera Compare

Litera documents text redlines, tracked Word output, and a separate page comparison for graphical PDFs. Its wider product includes synchronized review panes, Office integrations, and document-management services. [Litera product overview](https://info.litera.com/rs/046-QLX-552/images/Litera-Compare.pdf) · [Word review](https://support.litera.com/article/Reviewing-comparisons-in-Litera-Compare-for-Word-676506) · [PDF comparison modes](https://support.litera.com/article/Best-practice-for-comparing-PDF-files-in-Litera-Compare)

| Workflow | Browser implementation | Current boundary |
| --- | --- | --- |
| Word redline | Word-level insertions and deletions, previous/next edit, body text including table text, header/footer/note/comment text by package part | Existing revisions are read in their revised state. Numbering definitions, field evaluation, comment authorship, and existing revision history are outside the comparison. |
| Formatting, tables, moves | Direct paragraph/run properties in the body, row and cell structure including merge attributes, relocation of unique identical body paragraphs | Effective styles inherited from templates, repeated or edited moves, full table alignment, and page layout need additional coverage. |
| Word and PDF | Word-to-PDF and PDF-to-PDF body text, normalized wrapping, English OCR on pages without extractable text | PDF reading order follows the text layer. Mixed text and image pages can contain additional unrecognized words. OCR errors can create or obscure changes. |
| Graphical PDFs | Compare rendered pages by position with highlighted pixel differences and page additions/removals | Render resolution is capped at 1,400 pixels on the long edge. A page insertion can shift subsequent comparisons. |
| Excel | Named sheets, worksheet order, cell locations, values, types, formulas and cached results | Dates and numbers use their stored representation. Charts, spreadsheet recalculation, and rendered cell styling need additional coverage. |
| PowerPoint | Slide text by presentation order; embedded Office media bytes compared by package path | Slide masters, animations, chart rendering, and the placement/cropping of the same image need additional coverage. |
| Share results | Download a newly generated Word report with tracked text edits, or print the visible review to PDF | The Word report contains extracted content and section labels. Original document layout and formatting are not reconstructed in the report. |
| Enterprise use | Browser file pickers and local drag-and-drop | Office add-ins, DMS integrations, one-to-many comparisons, and PDF/A export are outside this implementation. |

## Published reference benchmark

Dechert published a redline for PTE 2020-02 made by Litera Compare for Word 11.4.0.111. The report identifies its input files and comparison settings. The browser regression suite reconstructs the short clause in section I(c), page 2, and checks both marked replacements and keyboard-focus navigation to each edit. The inputs for this check were reconstructed from the published PDF. [Dechert redline](https://www.dechert.com/content/dam/dechert%20files/services1/practice-areas/employee-benefits-and-executive-compensation/litera-compare-redlines/Litera%20Compare%20Redline%20-%20PTE%202020-02%20%28Current%29%20and%20PTE%202020-02%20%282024%29.pdf)

The broader benchmark uses the same browser comparison function on colored text runs extracted from the PDF. It selects physical lines with retained black text, deleted red words and inserted blue words. It excludes move colors and the summary page, normalizes each run to words, and evaluates both sides by word position. A line matches exactly only when every insertion and deletion mark agrees on both sides. Identical wording can still have different boundaries when one engine marks a whole phrase.

| Measure | Result |
| --- | ---: |
| Eligible lines with complete word-mark agreement | 61 / 76 |
| Litera-marked inserted word positions also marked by Diff Tool | 238 / 252 |
| Additional inserted word positions marked by Diff Tool | 5 |
| Litera-marked deleted word positions also marked by Diff Tool | 178 / 193 |
| Additional deleted word positions marked by Diff Tool | 6 |

These figures measure text-mark agreement on one reconstructed sample. They do not measure legal accuracy, full-document reading order, punctuation, formatting, moves, or fidelity to Litera's 429-change summary. The benchmark pins the PDF by SHA-256 and fails if the eligible population changes or exact agreement or per-side F1 falls below this baseline.

From the repository root, run the following with Node 22.18 or newer. The reference benchmark also needs Poppler's `pdftohtml` and a local copy of the linked PDF.

```sh
npm ci
npm run build
npx playwright test tests/e2e/document_comparison.spec.ts
node scripts/benchmark-litera.mjs /path/to/dechert-redline.pdf
```

The browser tests use local fixtures, including a searchable PDF exported from the repository's revised Word fixture and a one-page image-only PDF with the same sentence. They exercise uploads, drag-and-drop, a corrupt input followed by recovery, formula-only changes, paragraph moves, direct Word formatting, tracked report export, English OCR, and visual page comparison.
