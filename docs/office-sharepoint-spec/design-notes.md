# Office & SharePoint Design Notes

These notes extend the [Editorial Diff design language](../design.md) to
cover Office document comparison and SharePoint browsing. All UI should feel
consistent with the existing code diff and folder compare modes.

## Office Diff Screens

### Word Diff

**Layout:** Two-column reading layout. Left sidebar shows the document outline
(heading structure) as a navigable tree. Main content area shows change cards
in reading order.

**Change cards:** Each changed section (paragraph, table, heading) appears as
a card with:
- Section label ("Section 3: Project Overview")
- Change badge (Modified, Added, Deleted)
- Before/after text side-by-side for text changes
- Table changes rendered as compact grid with highlighted cells
- Comment changes in a distinct callout style

**Annotations rail:** A narrow right margin shows change markers aligned to
the document's reading position. Small colored dots or ticks indicate where
changes occur.

**Inspiration:** Manuscript proofing marks, Word's own "Track Changes" review
pane, but cleaner and more structured.

### PowerPoint Diff

**Layout:** Slide rail on the left (thumbnail-sized cards with slide numbers
and change badges). Main area shows the selected slide's changes in detail.

**Slide cards:**
- Slide number and title
- Change badge (Modified, Added, Deleted, Reordered)
- Mini summary: "Title changed, 3 text boxes modified"
- Click to expand full slide diff

**Slide detail view:**
- Title change: old → new, side-by-side
- Text box changes: labeled by shape name or position
- Speaker notes: collapsible section
- Table/image changes: thumbnails (future) with metadata summary

**Reordering:** A visual indicator (arrow or position badge) shows when a
slide moved from position 5 to position 8.

### Excel Diff

**Layout:** Sheet picker tabs at the top. Below, a document-like table view
showing changed cells in a compact grid.

**Sheet tabs:**
- Sheet name with change count badge ("Budget (14 changes)")
- Added/deleted sheets in muted tabs with distinct styling

**Grid view:**
- Compact cell grid showing only changed cells (not the full worksheet)
- Each cell shows: address (e.g., "C12"), value change (old → new)
- Formula changes highlighted with distinct background
- Row/column insertions shown as structural annotations
- Range filter bar: "Showing changes in A1:Z100. [Edit range]"

**Navigation:** Jump to next/previous changed cell. Sheet switcher dropdown.
Summary panel showing change counts by type.

## SharePoint Browser

### Layout

**Sidebar:** A slide-out panel or left-side pane (replacing or adjacent to
the file tree). Similar density to the existing file tree component.

**Three-level hierarchy:**
1. Sites — top level, with site name and URL
2. Drives (document libraries) — nested under sites
3. Files and folders — standard tree with expand/collapse

**Status indicators:**
- Green dot: connected, token valid
- Yellow dot: token expiring soon, refreshing
- Red dot: disconnected or auth error
- Grey dot: not connected

### Connection Flow

1. User clicks "Connect SharePoint" button (prominent, in settings or toolbar)
2. System browser opens Microsoft login
3. User authenticates and grants consent
4. Redirect back to app
5. Connection status updates, sites load automatically

### Version Picker

When a file is selected in the SharePoint browser:
- Detail panel shows file metadata (size, modified by, date)
- Version history displayed as a timeline or compact list
- Each version shows: version number, date, modified by, size
- Major versions distinguished from minor (auto-save) versions
- Select two versions → "Compare" button enables
- "Compare with local file" button for server-vs-local diff

### Review Queue

A "Shared With Me" or "Recent Changes" view:
- Aggregated list of documents recently modified by collaborators
- Grouped by: today, this week, earlier
- Each entry: file name + type icon, modified by + avatar/initials, time, change count
- Click to diff against the user's last viewed version (tracked locally)

## Component States

### Loading States

Office extraction and diff should feel like document processing, not a
generic spinner:

- **Word:** "Preparing document..." → "Extracting sections..." → "Comparing..."
- **PowerPoint:** "Loading presentation..." → "Analyzing 24 slides..." → slide cards appear progressively
- **Excel:** "Loading workbook..." → "Processing 8 sheets..." → "Comparing cells in Budget..."
- **SharePoint:** "Connecting to SharePoint..." → "Loading sites..." → "Browsing Marketing library..."

Use the editorial "document loading" metaphor from the design language:
paper-like surfaces appearing, content filling in from top to bottom.

### Empty States

- **No Office file selected:** "Open a Word, PowerPoint, or Excel file to compare."
- **Identical documents:** "These documents are identical." with a green checkmark.
- **SharePoint not connected:** "Connect SharePoint to browse document libraries and compare versions." with connect button.
- **No versions available:** "This file has no previous versions in SharePoint."
- **No recent changes:** "No recent changes from your collaborators."

### Error States

- **Corrupt Office file:** "This file appears to be corrupt or not a valid Office document. [Details]"
- **Unsupported format:** ".doc files (legacy Word format) are not supported. Save as .docx to compare."
- **SharePoint auth error:** "Could not connect to SharePoint. Your session may have expired. [Reconnect]"
- **SharePoint rate limited:** "SharePoint is temporarily limiting requests. Retrying in 30 seconds..."
- **Network offline:** "Cannot reach SharePoint. Check your internet connection. [Retry]"

## Motion and Transitions

Office diff components should follow the same restrained motion principles
as the core Editorial Diff language:

- Slide cards appear with a subtle reveal (top to bottom, staggered)
- Section expansion uses a height transition (not a pop-in)
- SharePoint browser slides in from the left as a drawer
- Version timeline animates with a simple list expansion
- Change cards highlight briefly when scrolled to (scroll-triggered)

No bouncy, springy, or playful animations. Keep it editorial and precise.

## Typography

Office document content should use typography that distinguishes it from code:

- **Word text:** Use the serif or serif-like display face defined in the
  Editorial Diff typography system. Document content should feel like a
  manuscript or proofing sheet.
- **PowerPoint titles:** Slightly larger, bolder weight. Slide titles should
  have more visual presence than body text.
- **Excel cells:** Monospace for values and formulas. Tabular number alignment.
  Cell addresses in a smaller, muted mono face.
- **SharePoint metadata:** Sans-serif for UI chrome (file names, dates, user
  names). Keep it compact and scannable.

## Accessibility

- All change badges use both color and icon/shape to indicate status
  (not color alone).
- SharePoint browser supports keyboard navigation (arrow keys, Enter to
  expand, Space to select).
- Version picker is keyboard-operable.
- Screen reader announcements for connection status changes.
- Focus management: when the SharePoint browser opens, focus moves to the
  first site. When comparison completes, focus moves to the first change.
