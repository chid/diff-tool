# Browser document comparison

Open **Documents**, choose an original and a revision, or drop two files on the app in that order. The direct route is `?tab=documents`. Document contents, PDF rendering, and English OCR stay in the browser. The app serves the OCR worker, engine, and language model from its own assets on demand.

## Write a comparison report

After comparison finishes, select **Download review report**. The report names the original and revision, summarizes the observed differences, and includes evidence from the comparison. For text and Office documents it quotes changed passages and records detected moves or media changes. For rendered PDFs it identifies changed, added, and removed pages and embeds their preview images. The report follows the selected comparison mode.

Open the downloaded HTML in a browser to read it offline or use the browser's Print command to save a PDF. The report is generated on-device from the comparison results. The separate **Download tracked Word report** action produces a DOCX for accepting or rejecting text edits. [Example contract report](https://chid.github.io/diff-tool/document-demos/supplier-report.html) · [Example launch report](https://chid.github.io/diff-tool/document-demos/launch-report.html) · [Example floor plan report](https://chid.github.io/diff-tool/document-demos/reading-room-report.html)

## Review AI governance changes

Each case follows one question through two dated company publications. The redline compares two short, verbatim excerpts addressing that question. Each has a dated primary-source link and a separate analyst note. Use the client question to identify evidence about a particular deployment. The cases are fixed historical snapshots. Subsequent policy changes require a new review.

| Document and dates | Question for the review | App and printable HTML |
| --- | --- | --- |
| Google AI Principles · June 2018 and February 2025 | How can someone challenge an AI decision, and who handles it? | [Open Google review](https://chid.github.io/diff-tool/?tab=documents&example=google) · [Google report](https://chid.github.io/diff-tool/document-demos/google-governance-report.html) |
| Claude’s Constitution · May 2023 and January 2026 | What should Claude do when a user’s request conflicts with an authorized supervisor’s instruction? | [Open Anthropic review](https://chid.github.io/diff-tool/?tab=documents&example=anthropic) · [Anthropic report](https://chid.github.io/diff-tool/document-demos/anthropic-governance-report.html) |
| OpenAI Model Spec · September and October 2025 | How should an assistant respond to loneliness while supporting the user’s relationships with other people? | [Open OpenAI review](https://chid.github.io/diff-tool/?tab=documents&example=openai) · [OpenAI report](https://chid.github.io/diff-tool/document-demos/openai-governance-report.html) |

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
| Share results | Download a self-contained HTML review report, a Word report with tracked text edits, or print the visible text review to PDF | The HTML report covers the selected mode. The Word report contains extracted content and section labels in a new document. |
| Enterprise use | Browser file pickers and local drag-and-drop | Office add-ins, DMS integrations, one-to-many comparisons, and PDF/A export are outside this implementation. |

## Published reference benchmark

Dechert published a redline for PTE 2020-02 made by Litera Compare for Word 11.4.0.111. The report identifies its input files and comparison settings. The browser regression suite reconstructs the short clause in section I(c), page 2, and checks both marked replacements and keyboard-focus navigation to each edit. The inputs for this check were reconstructed from the published PDF. [Dechert redline](https://www.dechert.com/content/dam/dechert%20files/services1/practice-areas/employee-benefits-and-executive-compensation/litera-compare-redlines/Litera%20Compare%20Redline%20-%20PTE%202020-02%20%28Current%29%20and%20PTE%202020-02%20%282024%29.pdf)

The broader benchmark uses the same browser comparison function on colored text runs extracted from the PDF. It selects physical lines with retained black text, deleted red words and inserted blue words. It excludes move colors and the summary page, normalizes each run to words, and evaluates both sides by word position. A line matches exactly only when every insertion and deletion mark agrees on both sides. Identical wording can still have different boundaries when one engine marks a whole phrase.

A second redline was held back until the alignment change was implemented. It uses the same publisher, Litera version, and line-selection rules, with a different pair of documents. [Dechert redline for 2022-15702 and 2024-06059](https://www.dechert.com/content/dam/dechert%20files/knowledge/onpoint/2024/4/Litera%20Compare%20Redline%20-%202022-15702%20and%202024-06059.pdf)

| Measure | PTE before | PTE after | Second redline before | Second redline after |
| --- | ---: | ---: | ---: | ---: |
| Lines with exact agreement | 61 / 76 | 63 / 76 | 40 / 55 | 41 / 55 |
| Reference insertions also marked | 238 / 252 | 239 / 252 | 157 / 179 | 159 / 179 |
| Extra inserted word positions | 5 | 0 | 8 | 3 |
| Reference deletions also marked | 178 / 193 | 180 / 193 | 143 / 162 | 145 / 162 |
| Extra deleted word positions | 6 | 0 | 5 | 0 |

The benchmark pins each PDF by SHA-256 and checks exact lines, matched positions, and extra marks independently. Scores apply to reconstructed word marks on the eligible lines. Some lines include list labels with identical original and revised text that Litera marked as replacements. Punctuation, moves, formatting, full-document layout, and legal significance fall outside these measurements.

## Comparison speed

The browser compares corresponding edited paragraphs directly when their order and surrounding text support that pairing. When paragraphs move, exact paragraph matches anchor the word comparison. Word alignment is checked against both inputs so whitespace changes survive accepting or rejecting the report. Selecting a new revision reuses the successfully parsed original while that file is still selected, including any OCR result.

The benchmark below compares 1,500 synthetic clauses, each with one occurrence of the word being replaced. Results are median comparison times from seven measured runs after one warm-up on `u.local`, Node 22.23.3. These timings exclude file parsing, OCR and rendering. Both versions reproduced the complete original and revision and retained the same proportion of original characters in all three scenarios.

| Changed clauses | Before (734f9b5) | After |
| --- | ---: | ---: |
| One | 9.0 ms | 1.4 ms |
| Every tenth | 21.8 ms | 3.8 ms |
| Every clause | 842.2 ms | 23.3 ms |

From the repository root, run the following with Node 22.18 or newer. The reference benchmark also needs Poppler's `pdftohtml` and local copies of the two linked PDFs.

```sh
npm ci
npm run build
npx playwright test 'document_.*spec.ts'
node scripts/benchmark-litera.mjs /path/to/pte-2020-02-redline.pdf
node scripts/benchmark-litera.mjs /path/to/2022-15702-and-2024-06059.pdf
node scripts/benchmark-document-redline.mjs
```

The browser tests use local fixtures, including a searchable PDF exported from the repository's revised Word fixture and a one-page image-only PDF with the same sentence. They exercise uploads, drag-and-drop, a corrupt input followed by recovery, formula-only changes, paragraph moves, direct Word formatting, tracked report export, English OCR, and visual page comparison.
