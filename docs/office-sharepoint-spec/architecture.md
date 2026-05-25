# Office & SharePoint Architecture

## Crate Design

### New Crates

```
crates/
  office-adapter/         Office format detection, extraction, and diff
  sharepoint-adapter/     SharePoint OAuth, Graph API, version fetching
```

### Crate Dependency Graph

```
                    ┌──────────────┐
                    │  diff-cli    │
                    └──────┬───────┘
                           │ uses
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
  ┌──────────────┐ ┌──────────────┐ ┌──────────────────┐
  │ office-      │ │ sharepoint-  │ │ ai-adapter       │
  │ adapter      │ │ adapter      │ │ (planned)        │
  └──────┬───────┘ └──────┬───────┘ └────────┬─────────┘
         │                │                  │
         ▼                ▼                  ▼
  ┌──────────────────────────────────────────────────┐
  │                   diff-core                       │
  │  (text diff, hunk generation, intraline spans)    │
  └──────────────────────────────────────────────────┘
```

- `office-adapter` depends on `diff-core` for text-level comparison within
  extracted text blobs.
- `sharepoint-adapter` depends on `office-adapter` for diff computation on
  fetched Office files. It also depends on a shared HTTP client.
- `diff-cli` uses both adapters for `dt office` and `dt sharepoint` commands.
- `ai-adapter` (planned) sits alongside, enriching Office and SharePoint diff
  results with AI summaries, risk scores, and clustering.

## `office-adapter` Internals

### Module Structure

```
crates/office-adapter/
  Cargo.toml
  src/
    lib.rs                  # Public API, OfficeDiffRequest/Result types
    format.rs               # Format detection (extension + zip signature)
    unpack.rs               # Safe zip reading with size/part limits
    word/
      mod.rs                # Word document model
      extract.rs            # XML → structured WordDocument
      compare.rs            # WordDocument → Vec<OfficeSectionDiff>
    powerpoint/
      mod.rs                # PowerPoint document model
      extract.rs            # XML → structured Presentation
      compare.rs            # Presentation → Vec<OfficeSectionDiff>
    excel/
      mod.rs                # Excel document model
      extract.rs            # XML → structured Workbook
      compare.rs            # Workbook → Vec<OfficeSectionDiff>
    models.rs               # Shared types: OfficeFormat, OfficeSectionKind, etc.
    xml_util.rs             # XML parsing helpers (shared across formats)
```

### Dependencies

```toml
[dependencies]
diff-core = { path = "../diff-core" }
zip = "2"                    # Safe zip reading with size limits
quick-xml = "0.37"           # or roxmltree for XML parsing
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"                # Content hashing for images
chrono = "0.4"               # Timestamps for metadata
thiserror = "2"
tracing = "0.1"              # Instrumentation/logging
```

### Key Library Decisions

- **XML parser:** `quick-xml` for streaming extraction (handles large Excel
  sheets without loading full DOM). `roxmltree` as an alternative for
  document-tree-based formats (Word, PowerPoint) where random access to
  XML structure is needed.
- **Zip reader:** `zip` crate with explicit size/part limit checks before
  decompression.

### Format Detection

```rust
pub fn detect_format(path: &Path) -> Result<OfficeFormat> {
    // 1. Check extension
    match path.extension().and_then(|e| e.to_str()) {
        Some("docx") => Ok(OfficeFormat::Word),
        Some("pptx") => Ok(OfficeFormat::PowerPoint),
        Some("xlsx") => Ok(OfficeFormat::Excel),
        _ => return Err(OfficeError::UnknownFormat),
    }

    // 2. Verify zip signature (PK\x03\x04)
    // 3. Verify expected content types
    // 4. Return detected format
}
```

### Extraction Pipeline

```
Raw .docx file
  → zip::ZipArchive (with size limits)
    → word/document.xml  → Paragraphs, runs, text, tables
    → word/comments.xml  → Comments
    → word/styles.xml    → Heading styles
    → word/header*.xml   → Header content
    → word/footer*.xml   → Footer content
    → word/media/*       → Image hashes only
    → [Content_Types].xml → Part type mapping
  → WordDocument (structured model)
```

Similar pipelines for `.pptx` (slide XML parts) and `.xlsx` (sheet XML parts
+ shared strings table).

### Comparison Strategy

1. **Extract structured models** from left and right files.
2. **Match sections** by ID (paragraph index, slide ID, sheet name).
3. **Compare matched sections:**
   - For text: extract plain text, diff with `diff-core`.
   - For structure: compare counts, order, and metadata directly.
   - For formulas: compare formula text strings.
4. **Detect additions/deletions/reordering:**
   - Sections present in right but not left → `Added`
   - Sections present in left but not right → `Deleted`
   - Sections in different positions → `Reordered`
5. **Emit `OfficeDiffResult`** with per-section change details.

### Safety Guards

```rust
pub struct OfficeDiffOptions {
    pub max_part_size: usize,       // Default: 50 MB
    pub max_part_count: usize,      // Default: 10,000
    pub max_uncompressed_size: usize, // Default: 200 MB total
}
```

- Decompression bombs are caught by `max_part_size` and total size tracking.
- Part count limits prevent zip-bomb attacks with millions of tiny files.
- XML parsing is bounded — no recursive entity expansion (`Billion Laughs`
  attack prevented by parser configuration).

## `sharepoint-adapter` Internals

### Module Structure

```
crates/sharepoint-adapter/
  Cargo.toml
  src/
    lib.rs                  # Public API, SharePointDiffRequest, connection types
    auth.rs                 # OAuth 2.0 PKCE flow, token management
    client.rs               # Microsoft Graph API HTTP client
    sites.rs                # Site and drive listing
    files.rs                # File/folder browsing
    versions.rs             # Version history
    cache.rs                # Local file cache for fetched documents
    models.rs               # SharePoint-specific types
```

### Dependencies

```toml
[dependencies]
office-adapter = { path = "../office-adapter" }
reqwest = { version = "0.12", features = ["json", "stream"] }
oauth2 = "5"                 # OAuth 2.0 PKCE flow
keyring = "3"                # OS keychain for token storage
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
thiserror = "2"
tracing = "0.1"
url = "2"                    # URL construction for Graph API
directories = "5"            # XDG cache directory resolution
```

### Data Flow: Version Diff

```
User selects two versions in UI
  │
  ▼
React calls sharepoint_diff_versions()
  │
  ▼
sharepoint-adapter checks cache
  ├── Both versions cached → read from disk
  └── Cache miss → fetch from Graph API
       │
       ├── GET /drives/{id}/items/{id}/versions/{vid}/content
       ├── Save to cache
       └── Read from cache
  │
  ▼
Both files passed to office-adapter::compare()
  │
  ▼
OfficeDiffResult returned to UI
  │
  ▼
React renders with format-specific navigation
```

### Data Flow: Server-vs-Local Diff

```
User selects local file + SharePoint file
  │
  ▼
React calls sharepoint_diff_local_vs_server()
  │
  ▼
sharepoint-adapter:
  1. Read local file bytes
  2. Fetch server version (or use cache)
  3. Pass both to office-adapter::compare()
  │
  ▼
OfficeDiffResult returned to UI
```

### OAuth Implementation

```rust
pub struct SharePointAuth {
    client_id: String,           // From app registration
    authority: String,           // https://login.microsoftonline.com/common
    redirect_uri: String,        // http://localhost:{port} or custom protocol
    scopes: Vec<String>,         // Sites.Read.All, Files.Read.All, offline_access
}

impl SharePointAuth {
    /// Start the OAuth flow. Opens system browser.
    pub async fn authorize(&self) -> Result<StoredToken>;

    /// Exchange refresh token for new access token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<StoredToken>;

    /// Get current valid access token (refreshing if needed).
    pub async fn get_access_token(&self) -> Result<String>;

    /// Revoke tokens and clear stored credentials.
    pub async fn disconnect(&self) -> Result<()>;
}
```

The `keyring` crate is used for token storage:

```rust
// Store
keyring::Entry::new("com.diff-tool.sharepoint", &user_id)?
    .set_password(&serialized_token)?;

// Retrieve
let token = keyring::Entry::new("com.diff-tool.sharepoint", &user_id)?
    .get_password()?;

// Delete on disconnect
keyring::Entry::new("com.diff-tool.sharepoint", &user_id)?
    .delete_password()?;
```

## Tauri Bridge

### Desktop UI Architecture

```
┌────────────────────────────────────────────────┐
│                 React UI                        │
│  ┌──────────┐ ┌──────────┐ ┌────────────────┐  │
│  │FileTree  │ │DiffPane  │ │SharePointBrowser│  │
│  │(source   │ │(Office   │ │(sites, drives,  │  │
│  │ selector) │ │ renderer)│ │ files, versions)│  │
│  └────┬─────┘ └────┬─────┘ └───────┬────────┘  │
│       │            │               │            │
├───────┼────────────┼───────────────┼────────────┤
│       │    Tauri IPC Commands      │            │
│       ▼            ▼               ▼            │
│  ┌──────────────────────────────────────────┐   │
│  │              Rust Backend                 │   │
│  │  office-adapter  sharepoint-adapter       │   │
│  └──────────────────────────────────────────┘   │
└────────────────────────────────────────────────┘
```

### Settings

New settings added to `settings-registry.json`:

```json
{
  "id": "office.maxPartSize",
  "label": "Max part size (MB)",
  "type": "number",
  "default": 50,
  "category": "performance",
  "description": "Maximum uncompressed size per XML part in Office files."
},
{
  "id": "office.includeFormatting",
  "label": "Detect formatting changes",
  "type": "toggle",
  "default": false,
  "category": "fileDisplay",
  "description": "Include formatting change detection in Office diffs."
},
{
  "id": "sharepoint.autoRefresh",
  "label": "Auto-refresh token",
  "type": "toggle",
  "default": true,
  "category": "performance",
  "description": "Automatically refresh SharePoint access tokens before expiry."
},
{
  "id": "sharepoint.cacheTtlHours",
  "label": "Cache TTL (hours)",
  "type": "number",
  "default": 24,
  "category": "performance",
  "description": "How long to cache downloaded SharePoint files."
},
{
  "id": "sharepoint.cacheMaxMb",
  "label": "Cache max size (MB)",
  "type": "number",
  "default": 500,
  "category": "performance",
  "description": "Maximum disk space for the SharePoint file cache."
}
```

## Security Review Points

- [ ] OAuth PKCE implementation verified — no client secret in desktop app
- [ ] Redirect URI uses localhost with random port or custom protocol
- [ ] Access tokens never logged or serialized to disk
- [ ] Refresh tokens stored exclusively in OS keychain
- [ ] Zip bomb protection tested with adversarial fixtures
- [ ] XML entity expansion disabled in parser configuration
- [ ] SharePoint file cache uses restrictive file permissions (0600)
- [ ] Rate limiting honored with Retry-After header parsing
- [ ] No SharePoint data sent to any third-party service (including AI
      endpoints, unless user explicitly opts in)
