# Document review feature roadmap

The next document review release should help a reviewer answer “Where did this change come from?” in a contract, drawing set, or slide deck. The browser already marks words, compares PDF pages by position, reads slide text in presentation order, and exports a tracked Word report. The proposals below extend that browser workflow. [Current coverage and benchmark](document-comparison.md) · [Architecture and runtime boundaries](architecture.md)

The numbers give the proposed delivery order. Dependencies refer to those numbers. The relative effort estimates use **S** for a bounded UI or comparison rule, **M** for work spanning a model and a workflow, and **L** for a larger matching, rendering, or export capability.

## First milestone — trace a change to its source

Ship features 1–3 together. A reviewer should be able to locate a changed term in the supplier agreement, recognize the same drawing after a page insertion, and follow a launch slide that changed position.

| Priority | Feature and reviewer outcome | Acceptance test | Depends on | Effort |
| --- | --- | --- | --- | --- |
| 1 | **Source locations on edits.** Open a change at the original or revised passage. | Reach the changed payment term at its DOCX paragraph, a PDF phrase at its page and coordinates, and a slide title at its shape. Test a repeated phrase so the selected occurrence must be correct. Show the matching side for one-sided edits. | — | M |
| 2 | **Match PDF pages across insertions.** Keep the reviewer on the same drawing when a cover sheet is added or a plan moves. | In a four-page fixture, prepend a cover, reorder two identifiable sheets, and edit one of them. Report one addition and highlight the edit on the corresponding sheet. For indistinguishable duplicates, allow a reviewer to choose the pairing. | 1 | L |
| 3 | **Match slides across revisions.** Follow an edited slide through a deck reorder or file repack. | In a four-slide deck, move the pricing slide, edit its price, and regenerate its package part name. Report a move and the single price edit. Distinguish duplicate titles using slide content or flag the pairing for review. | 1 | M |

Start by extending the parsed document with source references alongside its ordered sections. Preserve the current text spans and their accept/reject reconstruction tests. PDF and PowerPoint can then attach their own pairing results to those references. Use the existing Word, PDF, and slide demos for a manual walkthrough and keep the ambiguous cases in dedicated fixtures. [Readers](../apps/desktop/src/documents/read.ts) · [Redline engine](../apps/desktop/src/documents/redline.ts)

## Next milestone — review and record decisions

| Priority | Feature and reviewer outcome | Acceptance test | Depends on | Effort |
| --- | --- | --- | --- | --- |
| 4 | **Synchronized original and revised views.** Read a change with surrounding content on both sides. | Selecting a change opens the corresponding excerpt on each side. Moving to the next edit scrolls both views. A missing paragraph or page shows a gap on the missing side. Keyboard navigation retains focus. | 1–3 | M |
| 5 | **Filter changes by meaning.** Find changed amounts, dates, and obligations in a long revision. | In a fixture with a price increase, changed date, deleted “must”, and a spelling correction, each filter finds its matching edits. Clearing filters restores every edit. Show the rule that matched each result. | 1 | S |
| 6 | **Record a decision per change.** Mark edits as accepted, disputed, or still open and attach a reviewer note. | Decide one of two identical-looking edits and leave the other open. Filter to open edits, revisit the decision, and verify counts and note ownership by edit. | 1 | M |
| 7 | **Save and reopen a review locally.** Resume work or hand off recorded decisions. | Export a review, close the app, and import it with the same two source files. Restore notes, decisions, and manual pairings. Replacing either source with different bytes under the same filename must prompt for reconciliation. | 2, 3, 6 | M |

## Format-specific fidelity

| Priority | Feature and reviewer outcome | Acceptance test | Depends on | Effort |
| --- | --- | --- | --- | --- |
| 8 | **Align Word table rows by key.** Identify an amended quantity even when someone reorders the schedule. | Move the row for one SKU, insert another, and change one quantity. Identify the move, addition, and edited cell under the correct column. Duplicate keys trigger a choice or an ambiguity marker. | 1 | M |
| 9 | **Export tracked edits in a copy of the source DOCX.** Keep the contract's styles, numbering, images, and headers when handing it back to the author. | Begin with body-text edits in a styled contract. Open the export in Word and LibreOffice. Accepting all yields the revised passage, rejecting all yields the original, and an untouched header, image, and numbered list survive. Identify unsupported edits before export. | 1 | L |
| 10 | **OCR scanned regions on mixed PDF pages.** Find an edit in a scanned table even when the page has a searchable heading. | Change an amount present only in a scanned region beneath a native-text heading. Report the amount once, preserve the heading once, and identify the recognized region for visual checking. | 1 | L |
| 11 | **Inspect drawing changes at full detail.** Zoom a selected region and toggle original and revised drawings in register. | Use aligned drawings with a fine line alteration too small to distinguish in the overview. At higher zoom, render both crops from the source PDFs and reveal the altered line. Preserve the selected region when the sheet moves. | 1, 2 | M |
| 12 | **Detect changed placement and crops in PowerPoint.** Catch a shifted chart image or a cropped product photo even when its bytes stay the same. | Move and crop one image on a slide while leaving another untouched. Show the two changed properties on the right shape with original and revised bounds. A package-only image rename yields no visual change. | 1, 3 | M |
| 13 | **Compare native PowerPoint chart data.** Identify the series and category behind a changed bar or point. | In a two-series chart, change one quarter's value and swap series order. Show the old and new value for that quarter and identify the reorder. An unchanged value under a different series must keep its own identity. | 1, 3 | M |
| 14 | **Inspect pre-existing Word revisions by author.** Tell inherited markup from edits introduced in this comparison. | Compare a marked contract with a copy containing one new tracked replacement. Surface the existing revision's author and text, label the new replacement, and preserve the current final-text comparison as a selectable view. | 1 | M |

## Reconcile parallel revisions

| Priority | Feature and reviewer outcome | Acceptance test | Depends on | Effort |
| --- | --- | --- | --- | --- |
| 15 | **Three-way clause review.** Compare two independently revised contracts against their common draft. | Edit separate clauses on the two branches and change the same payment term differently on both. Show both independent edits and one conflict with the base and both proposals. Record the chosen resolution and restore it when the review reopens. | 1, 6, 7 | L |

**Release decision.** After the first milestone, use the three demo workflows to choose the next investment. If reviewers can locate edits but lose their place, prioritize synchronized views. If the handoff fails because counsel needs a styled contract back, move the source-preserving Word export forward. Keep the published word-mark benchmark as a regression check and measure pairing accuracy separately on reordered pages and slides. [Verification workflow](development.md#choose-a-verification-loop)
