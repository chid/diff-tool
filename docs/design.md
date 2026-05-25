# Design Language: Editorial Diff

Editorial Diff is a visual language for a professional diff tool that handles
code, folders, Office documents, images, and Git review workflows. The goal is
to feel like a modern proofing desk: precise, document-centric, and dense
enough for serious review work without becoming heavy or old-fashioned.

## Design Principles

- Treat content as the primary surface.
- Use whitespace intentionally, not generously.
- Make change state obvious through structure before color.
- Keep interaction controls compact and predictable.
- Prefer readable editorial cues over decorative chrome.
- Support long review sessions without fatigue.

## Visual Language

### Overall Feel

- Quiet, editorial, and technical.
- Inspired by print proofing, manuscript review, and document comparison.
- More structured and document-like than a typical SaaS dashboard.
- Strong hierarchy, restrained motion, and confident spacing.

### Layout

- Two- and three-pane review layouts with fixed structural rails.
- Persistent left navigation or file tree.
- Main content area uses a reading-column rhythm rather than card grids.
- Secondary inspector panels should dock cleanly rather than float like modal
  widgets.
- Avoid decorative hero sections and marketing composition.

### Typography

- Use a high-legibility serif or serif-like display face for headings and a
  clean sans-serif or mono for data, code, and controls.
- Keep body text compact but readable.
- Use typographic hierarchy to distinguish document content, metadata, and
  review controls.
- Headlines should feel editorial, not corporate.

### Color

- Base palette: paper white, warm off-white, charcoal, ink, muted slate.
- Accent palette:
  - additions: green-teal
  - deletions: muted red
  - modifications: blue or blue-gray
  - warnings: amber
  - selection: deep indigo or ink-blue
- Use color as annotation, not wallpaper.
- Prefer soft fills, underlines, edge markers, and margin rails over loud
  blocks.

### Surfaces

- Paper-like content surfaces for documents and diff panes.
- Slightly elevated inspector panels.
- Thin borders and subtle shadows.
- No heavy glassmorphism or floating card stacks.

### Motion

- Use restrained transitions for pane changes, selection, and expansion.
- Motion should reinforce reading direction and hierarchy.
- Avoid playful or bouncy transitions.

## Component Language

### File Tree

- Dense list structure with status markers in the margin.
- Compact row height and strong path truncation rules.
- Selected rows should read as a highlighted line in a manuscript, not a
  pill-shaped chip.

### Diff Pane

- Side-by-side and unified modes should both feel like proofing sheets.
- Line numbers, annotations, and hunk headers should sit in a clear editorial
  rail.
- Additions and deletions should use understated but unmistakable background
  treatments.

### Office Diff

- Word: manuscript-style column with change annotations in the margin.
- PowerPoint: slide list with evidence cards and a compact per-slide summary.
- Excel: sheet/range navigation with document-like table blocks, not spreadsheet
  chrome overload.

### Merge

- Three-way merge should feel like a comparison desk with source columns and a
  result column.
- Conflict markers should be obvious but not garish.

### Settings

- Settings should read like an editorial preferences panel.
- Keep it compact, labeled, and dense.
- Group controls by review behavior, presentation, performance, and Office
  extraction policy.

## Screen Directions

### Home

- Recent comparisons as a document list.
- Quick open and recent workspace actions.
- Minimal promotional framing.

### File Diff

- Reading-column content area.
- Margin annotations for changes and hunk context.
- Clear split/unified toggle and review controls.

### Folder Compare

- Two-pane tree and summary rail.
- Status counts and filters in a narrow inspector.
- Fast scanning and obvious selection state.

### Merge Resolver

- Source panes, conflict summary, and result pane.
- Decision controls should be grouped near the current conflict.

### Office Diff

- Format-specific navigation with document-like result sections.
- Use structured summaries first, richer previews second.

### Performance State

- Show progress as a document loading and parsing state.
- Prefer clear parse stages over generic spinners.

## Prompt Pack

Use these prompts when generating wireframes, app flows, or visual explorations.

### Master Prompt

```text
Design a desktop application called Diff Tool using the "Editorial Diff" visual
language.

The app is a local-first, cross-platform diff and merge tool for developers and
technical reviewers. It compares code, folders, Git changes, images, and Office
documents (Word, PowerPoint, Excel). The UI should feel like a modern proofing
desk: editorial, precise, document-centric, and dense enough for serious review
work.

Core visual direction:
- paper-like surfaces
- strong typographic hierarchy
- thin borders and subtle shadows
- margin annotations and rails
- restrained color used as proofing marks
- compact controls and reading-column rhythm

Avoid:
- marketing-style cards
- decorative gradients
- playful motion
- oversized hero sections
- consumer app styling
- generic SaaS dashboards

Render these screens:
1. Home / recent comparisons
2. File diff view with side-by-side and unified modes
3. Folder compare view with tree navigation and status summary
4. Merge conflict resolver
5. Git changeset / review queue
6. Word / PowerPoint / Excel diff modes
7. Performance loading state
8. Settings panel

For each screen, show:
- a full desktop wireframe
- the main content hierarchy
- annotations for controls and state
- one conservative production direction
- one more expressive editorial direction

The app must feel fast, credible, and professional on macOS and Windows.
```

### Short Prompt For Iterations

```text
Apply the "Editorial Diff" visual language to this desktop diff tool. Keep it
editorial and document-like, with paper surfaces, margin annotations, thin
rules, muted color, and compact technical controls. Avoid SaaS cards and
decorative flair. Focus on dense, legible, production-ready wireframes and app
flows for code diff, folder compare, merge, and Office document review.
```

### Wireframe Prompt

```text
Create grayscale wireframes for the Diff Tool app in the "Editorial Diff"
language. Prioritize layout, hierarchy, annotation rails, pane structure, and
review flow. Do not use decorative imagery or stylized gradients. Show desktop
layouts only. Include labeled regions for file tree, diff content, hunk
navigation, merge choices, Office-specific sections, and settings.
```

### High-Fidelity Prompt

```text
Create a high-fidelity desktop UI concept for Diff Tool using the "Editorial
Diff" language. The interface should feel like a premium document proofing and
review workstation. Use paper-like surfaces, a restrained palette, elegant
typography, subtle shadows, and change annotations in the margin. Make the app
feel technically exact and calm rather than flashy. Show realistic content for
code diffs, Office diffs, and merge workflows.
```

## Do / Don’t

### Do

- Use editorial hierarchy.
- Make annotations easy to scan.
- Keep dense layouts readable.
- Make the product feel credible for long review sessions.
- Favor utility and clarity over novelty for novelty's sake.

### Don’t

- Use rounded marketing cards as the main layout motif.
- Add random decorative blobs or orbs.
- Use a generic dark-blue SaaS look.
- Over-animate the interface.
- Hide the actual content behind visual effects.

## Follow-Up Notes

- If the design language changes, update the prompt pack in the same file.
- If an image model prompt proves weak, create a narrower prompt section rather
  than replacing the whole document.
- If Office diff becomes a major pillar, extend the Office section before
  changing the core language.
