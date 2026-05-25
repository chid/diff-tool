# Office & SharePoint Integration — Overview

## Summary

Add first-class comparison for Microsoft Office files (Word, PowerPoint, Excel)
with optional SharePoint cloud sync. This transforms the diff tool from a
code-only comparison tool into a universal review workspace that covers the
full spectrum of documents developers and technical reviewers encounter:
source code, config files, images, Office documents, and cloud-synced
SharePoint libraries.

## Product USP

Most Office comparison tools are:

- tied to Microsoft Office and Windows,
- legal-document focused,
- Excel-only,
- cloud/API-first,
- or disconnected from developer review workflows.

Most diff tools (Kaleidoscope, Beyond Compare, Araxis) do not touch Office
formats or SharePoint at all.

This app can differentiate by:

1. **Universal workspace** — review `.docx`, `.pptx`, `.xlsx`, source code,
   images, and config files in one tool.
2. **Local-first, no Office required** — unpack and compare Office files
   natively using the Open Packaging Convention (zip + XML), with no
   dependency on Microsoft Office.
3. **SharePoint-aware** — connect to SharePoint document libraries, compare
   document versions, and diff server-vs-local copies.
4. **AI-enriched review** — combine structured Office diffs with AI change
   summaries, risk scoring, and intent clustering (see
   [ADR 0002](../decisions/0002-ai-features-byok.md)).
5. **Machine-readable output** — produce JSON diff models that feed CI,
   agents, and automated review pipelines.

## What's In Scope

### Office Formats

| Format | Extension | V1 Coverage |
|--------|-----------|-------------|
| Word | `.docx` | Paragraphs, headings, tables, comments, tracked changes, headers/footers, embedded image presence |
| PowerPoint | `.pptx` | Slide reordering, titles, speaker notes, text boxes, shapes, tables, charts, image presence |
| Excel | `.xlsx` | Sheets, used ranges, cell values, formulas, named ranges, comments, row/column insertions |

### SharePoint Sync

| Capability | Description |
|------------|-------------|
| Authentication | OAuth 2.0 via Microsoft Identity Platform (personal + organizational accounts) |
| Library browsing | List document libraries, folders, and files via Microsoft Graph API |
| Version diff | Compare any two versions of a document stored in SharePoint |
| Server-vs-local diff | Compare a local copy against the latest SharePoint version |
| Check-out awareness | Detect document locks and check-out status |

### Out Of Scope (V1)

- Legacy `.doc`, `.ppt`, `.xls` binary formats (OLE2)
- Real-time co-authoring integration
- Bidirectional sync (push local changes to SharePoint)
- SharePoint on-premises (2016/2019) — Graph API only
- Pixel-perfect redline rendering (structured change cards first)
- Office 365 web editor embedding

## Relationship To Existing Project

This spec builds on the architecture established in [ADR 0001](../decisions/0001-rust-core-tauri-shell.md)
and the implementation plan in [docs/implementation-plan.md](../implementation-plan.md).

```
Existing crates:           New crates:
  diff-core                  office-adapter
  git-adapter                sharepoint-adapter
  diff-cli
  ai-adapter (planned)
```

The `office-adapter` crate handles format detection, zip unpacking, XML
parsing, and structured diff generation. The `sharepoint-adapter` crate
handles OAuth, Graph API communication, version fetching, and caching.
Both feed into the existing `diff-core` for text-level comparison and the
planned `ai-adapter` for enrichment.

## Document Map

- [Overview](./overview.md) — this file
- [Office Diff Feature](./office-diff.md) — format support details, extraction, data models
- [SharePoint Sync](./sharepoint-sync.md) — authentication, API design, sync workflows
- [Architecture](./architecture.md) — crate design, data flow, security, caching
- [Implementation Plan](./implementation-plan.md) — phased sequence, milestones, dependencies
- [Design Notes](./design-notes.md) — UI design for Office and SharePoint modes
