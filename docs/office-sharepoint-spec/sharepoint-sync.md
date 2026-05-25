# SharePoint Sync Integration

## Summary

Add cloud-connected comparison for Office documents stored in SharePoint
Online. Users can browse SharePoint document libraries, compare document
versions, and diff server copies against local files — all from within
the same diff workspace.

This is an additive feature. The Office diff engine (see
[office-diff.md](./office-diff.md)) works fully offline. SharePoint
integration adds cloud connectivity as an optional enhancement.

## User Stories

### Primary Flows

1. **Browse and compare.** A user opens the SharePoint browser in the app,
   navigates to a document library, selects two files (or two versions of
   the same file), and sees a structured diff.

2. **Version history diff.** A user selects a document with 15 versions in
   SharePoint. They pick versions 7 and 14, and the tool fetches both,
   diffs them, and displays what changed.

3. **Server-vs-local diff.** A user has a local `.docx` file they've been
   editing. They connect to SharePoint, find the same document, and diff
   their local copy against the latest server version.

4. **Review queue.** A user opens a "Shared With Me" view showing documents
   recently modified by teammates. They click a document to see what changed
   since they last viewed it.

### Secondary Flows

5. **Conflict detection.** If a local file has edits and the server version
   has also changed, the tool flags a potential conflict and offers a 3-way
   merge view (local, server, last-common-version).

6. **AI-powered review.** After fetching and diffing a SharePoint document,
   the user clicks "Summarize with AI" to get a natural-language summary of
   what their coworker changed (requires AI BYOK configuration — see
   [ADR 0002](../decisions/0002-ai-features-byok.md)).

## Authentication

### Provider

Microsoft Identity Platform (Azure AD / Entra ID) via OAuth 2.0 authorization
code flow with PKCE.

### Account Types

| Account Type | Supported | Notes |
|---|---|---|
| Personal Microsoft account | ✅ V1 | `common` endpoint |
| Work/school (Entra ID) | ✅ V1 | Tenant-specific or `organizations` endpoint |
| Multi-tenant app registration | ✅ V1 | Admin consent flow for org-wide deployment |
| Sovereign clouds (GCC, etc.) | ❌ V2 | Requires separate endpoint configuration |

### Token Storage

API tokens are stored in the OS keychain using the `keyring` crate (same
mechanism as AI BYOK keys). Never stored in settings JSON, `localStorage`,
or source files.

- **Access token:** short-lived (1 hour), stored in memory only
- **Refresh token:** long-lived, stored in OS keychain
- **Token refresh:** transparent, handled by the backend

### OAuth Flow

```
User clicks "Connect SharePoint"
  → System browser opens Microsoft login page
  → User authenticates and grants consent
  → Redirect URI catches auth code (localhost callback or custom protocol)
  → Backend exchanges code for tokens
  → Tokens stored in keychain
  → Connection status shown in UI
```

### Required Scopes

| Scope | Purpose |
|-------|---------|
| `Sites.Read.All` | List sites and document libraries |
| `Files.Read.All` | Read file contents and metadata |
| `Sites.ReadWrite.All` | Optional: for future sync/push features |
| `offline_access` | Refresh token for long-lived sessions |

## API Design

### Microsoft Graph API

SharePoint Online is accessed through Microsoft Graph. Key endpoints:

```
# List sites
GET /v1.0/sites?search=*

# List document libraries for a site
GET /v1.0/sites/{site-id}/drives

# List files in a library
GET /v1.0/drives/{drive-id}/root/children
GET /v1.0/drives/{drive-id}/root:/path/to/folder:/children

# Get file metadata and download URL
GET /v1.0/drives/{drive-id}/items/{item-id}

# List versions of a file
GET /v1.0/drives/{drive-id}/items/{item-id}/versions

# Download file content
GET /v1.0/drives/{drive-id}/items/{item-id}/content

# Download specific version content
GET /v1.0/drives/{drive-id}/items/{item-id}/versions/{version-id}/content

# Search within a library
GET /v1.0/drives/{drive-id}/root/search(q='{query}')
```

### Caching Strategy

Fetched files are cached locally to avoid re-downloading the same version:

```
~/.cache/dt/sharepoint/
  {site-id}/
    {drive-id}/
      {item-id}/
        v{version-id}.docx
        v{version-id}.meta.json
```

- Cache key = `SHA256(site_id + drive_id + item_id + version_id)`
- Cache TTL = 24 hours by default, configurable
- Cache size limit = 500 MB by default, LRU eviction
- Manual "Clear SharePoint cache" button in Settings

## Data Models

### SharePoint-Specific Types

```rust
/// A SharePoint connection configuration.
pub struct SharePointConnection {
    pub tenant_id: String,
    pub user_display_name: String,
    pub user_email: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// A browsable SharePoint site.
pub struct SharePointSite {
    pub id: String,
    pub name: String,
    pub web_url: String,
    pub display_name: String,
}

/// A document library (drive) within a site.
pub struct SharePointDrive {
    pub id: String,
    pub name: String,
    pub drive_type: String,       // "documentLibrary"
    pub web_url: String,
    pub total_item_count: Option<u64>,
}

/// A file or folder item in a library.
pub struct SharePointItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_folder: bool,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
    pub modified_by: Option<String>,
    pub version_count: Option<u32>,
    pub checked_out_by: Option<String>,
}

/// A version of a file.
pub struct SharePointVersion {
    pub id: String,
    pub version_number: String,
    pub size: u64,
    pub last_modified: String,
    pub modified_by: String,
    pub is_major: bool,
}

/// Request to diff two SharePoint document versions.
pub struct SharePointDiffRequest {
    pub connection_id: String,
    pub item_id: String,
    pub left_version_id: String,
    pub right_version_id: String,
    pub office_options: OfficeDiffOptions,
}
```

### Tauri IPC Commands

```rust
// Connection management
#[tauri::command]
async fn sharepoint_connect() -> Result<SharePointConnection>;

#[tauri::command]
async fn sharepoint_disconnect() -> Result<()>;

#[tauri::command]
async fn sharepoint_connection_status() -> Result<Option<SharePointConnection>>;

// Browsing
#[tauri::command]
async fn sharepoint_list_sites() -> Result<Vec<SharePointSite>>;

#[tauri::command]
async fn sharepoint_list_drives(site_id: String) -> Result<Vec<SharePointDrive>>;

#[tauri::command]
async fn sharepoint_list_items(
    drive_id: String,
    folder_path: Option<String>,
) -> Result<Vec<SharePointItem>>;

#[tauri::command]
async fn sharepoint_search(
    drive_id: String,
    query: String,
) -> Result<Vec<SharePointItem>>;

// Versioning
#[tauri::command]
async fn sharepoint_list_versions(
    drive_id: String,
    item_id: String,
) -> Result<Vec<SharePointVersion>>;

// Diff
#[tauri::command]
async fn sharepoint_diff_versions(
    request: SharePointDiffRequest,
) -> Result<OfficeDiffResult>;

#[tauri::command]
async fn sharepoint_diff_local_vs_server(
    local_path: String,
    drive_id: String,
    item_id: String,
    version_id: Option<String>,  // None = latest
) -> Result<OfficeDiffResult>;
```

## CLI

```bash
# Connect to SharePoint
dt sharepoint connect

# List available sites
dt sharepoint sites

# List document libraries in a site
dt sharepoint drives --site-id <id>

# List files in a library
dt sharepoint ls --drive-id <id> [--path <path>]

# List versions of a file
dt sharepoint versions --drive-id <id> --item-id <id>

# Diff two versions
dt sharepoint diff \
  --drive-id <id> \
  --item-id <id> \
  --left-version <version-id> \
  --right-version <version-id> \
  --json

# Diff local file vs server version
dt sharepoint diff-local \
  --local <path> \
  --drive-id <id> \
  --item-id <id> \
  [--version <version-id>] \
  --json

# Disconnect
dt sharepoint disconnect
```

## Security Considerations

- **No data proxy.** All SharePoint communication goes directly from the
  user's machine to Microsoft Graph. No intermediate server.
- **Keychain storage.** OAuth tokens stored in OS keychain, never in plaintext
  config files.
- **Scope minimization.** Only request scopes actually needed. V1 uses
  read-only scopes.
- **Tenant filtering.** For organizational accounts, the user's tenant ID is
  recorded and used to scope API calls.
- **Cache encryption.** Cached SharePoint files are stored with restrictive
  file permissions (0600). Future enhancement: at-rest encryption for
  cached files on shared machines.
- **Disconnect clears state.** Disconnecting removes all cached tokens,
  cached files, and site metadata.

## Fail States

| State | Behavior |
|-------|----------|
| No connection | SharePoint features show "Connect to SharePoint" button |
| Token expired | Automatic refresh via refresh token; if refresh fails, prompt re-auth |
| Network error | "SharePoint unavailable. [Retry]" with exponential backoff |
| Permission denied | "You don't have access to this library. [Request Access]" |
| File checked out | "This file is checked out by [name]. Diff may not reflect latest changes." warning |
| Version not found | "Version [id] no longer exists. It may have been deleted." |
| Rate limited | Retry-After header honored; "SharePoint is rate-limiting. Retrying in [n] seconds." |

## Future Enhancements

- **Bidirectional sync.** Push local changes to SharePoint as new versions.
- **Check-out/check-in.** Full document reservation workflow.
- **Shared With Me view.** Aggregated view of recent collaborator changes.
- **SharePoint on-premises.** Support for SharePoint 2016/2019 via REST API.
- **Teams integration.** Link to Teams channels and files tabs.
- **OneDrive support.** Extend SharePoint browsing to personal OneDrive.
