# Tauri IPC Wiring Plan

Replace the hardcoded `sampleData.ts` mocks with live Tauri IPC calls so the
desktop app reads real files and directories from the filesystem.

## Current State

### What's mocked

| What | Mock | Real source |
|---|---|---|
| File tree entries | `sampleTree` (5 hardcoded paths) | `scan_tree` Tauri command → `git_adapter::scan_tree` |
| Diff content | `sampleOldFile` / `sampleNewFile` (2 hardcoded `FileContents`) | `compare_files` Tauri command → `diff_core::compare_files` |
| Diff stats | `sampleStats` (hardcoded numbers) | `DiffResult.stats` from Rust |

### IPC bridge (already exists)

```
tauri.ts                          main.rs
─────────────────────────────────────────────────────
compareFiles(left, right)  ──→   compare_files(request)
  → DiffResult                      → diff_core::DiffResult

scanTree(root)             ──→   scan_tree(root)
  → DirEntryDiff[]                  → Vec<git_adapter::DirEntryDiff>
```

### Type compatibility

Rust `DirEntryDiff` serializes directly into the TypeScript `DirEntryDiff`
interface — no conversion needed.

Rust `DiffResult` (with `FileDiff.hunks`, `LineChange`, etc.) does NOT match
`@pierre/diffs` `FileDiffMetadata`. The frontend uses `parseDiffFromFile` to
convert raw file contents into `FileDiffMetadata`, bypassing the Rust diff
engine entirely for rendering.

## The Gap

`parseDiffFromFile` needs `FileContents` (raw text), but the Rust
`compare_files` command only returns hunks/lines/stats — not the raw text.
We need the raw text to feed `@pierre/diffs`.

## Phase 1: Read File Contents (minimal viable wiring)

Add a `read_file_text` Tauri command. The frontend reads both files' text,
constructs `FileContents`, and passes through `parseDiffFromFile`.

### Step 1.1: Add `read_file_text` Rust command

**File:** `apps/desktop/src-tauri/src/main.rs`

```rust
#[tauri::command]
fn read_file_text(path: PathBuf, max_bytes: Option<u64>) -> Result<String, String> {
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let limit = max_bytes.unwrap_or(10 * 1024 * 1024) as usize;
    let slice = &bytes[..bytes.len().min(limit)];
    Ok(String::from_utf8_lossy(slice).into_owned())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            compare_files,
            scan_tree,
            read_file_text,  // ← new
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
```

### Step 1.2: Add TypeScript wrapper

**File:** `apps/desktop/src/tauri.ts`

```typescript
export async function readFileText(
  path: string,
  maxBytes?: number
): Promise<string> {
  return invoke<string>('read_file_text', { path, maxBytes });
}
```

### Step 1.3: Replace `sampleOldFile` / `sampleNewFile` in App.tsx

**File:** `apps/desktop/src/App.tsx`

Replace the `useMemo` that calls `parseDiffFromFile(sampleOldFile, sampleNewFile)`
with an async load that:

1. Calls `readFileText` for both left and right paths
2. Constructs `FileContents` objects
3. Calls `parseDiffFromFile`
4. Stores the result in state

```typescript
const [fileDiff, setFileDiff] = useState<FileDiffMetadata | null>(null);

useEffect(() => {
  async function load(left: string, right: string) {
    const [leftText, rightText] = await Promise.all([
      readFileText(left),
      readFileText(right),
    ]);
    const leftFile: FileContents = { name: left, contents: leftText };
    const rightFile: FileContents = { name: right, contents: rightText };
    setFileDiff(parseDiffFromFile(leftFile, rightFile));
  }
  load('/path/to/left', '/path/to/right');  // ← will come from file picker
}, []);
```

### Step 1.4: Add loading and error states

- Loading spinner while `readFileText` is in flight
- Error boundary / message for unreadable files
- Fallback to mock data when running in `npm run dev` (Vite) without Tauri

## Phase 2: Wire Live Folder Scanning

Replace `sampleTree` with real `scan_tree` output.

### Step 2.1: Call `scanTree` on app mount

```typescript
const [entries, setEntries] = useState<DirEntryDiff[]>([]);

useEffect(() => {
  scanTree('/path/to/repo').then(setEntries);
}, []);
```

### Step 2.2: Use Rust diff stats

Call `compareFiles` alongside `readFileText` to get real stats:

```typescript
const [stats, setStats] = useState({ additions: 0, deletions: 0, unchanged: 0 });

useEffect(() => {
  compareFiles(left, right).then((result) => setStats(result.stats));
}, [left, right]);
```

## Phase 3: Open File / Folder Flows

### Step 3.1: Tauri file open dialog

Use `@tauri-apps/plugin-dialog` for native open dialogs:

```typescript
import { open } from '@tauri-apps/plugin-dialog';

const selected = await open({
  multiple: false,
  filters: [{ name: 'All Files', extensions: ['*'] }],
});
```

### Step 3.2: Drag and drop

Tauri supports file drop events natively via `onDragDropEvent` from
`@tauri-apps/api/window`.

### Step 3.3: Command-line args

Read `std::env::args()` in `main.rs` to accept file/folder paths when
launched from the terminal.

## Phase 4: Use Rust Diff Data (future optimization)

Once the Rust diff engine produces multi-hunk, context-aware output with
intraline spans, consider sending the Rust `DiffResult` directly to the
frontend and rendering it without `@pierre/diffs`. OR, have Rust output in
a format `@pierre/diffs` can consume directly.

This is a future decision — the `parseDiffFromFile` approach works for now.

## Vite Dev vs. Tauri Detection

During `npm run dev`, Tauri commands will fail (no Rust backend). Detect
this and fall back to mock data:

```typescript
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

if (isTauri) {
  // live IPC
} else {
  // sampleData fallback
}
```

Alternatively, wrap each `invoke` call in a try/catch and fall back silently.

## Files Changed (expected)

| File | Change |
|---|---|
| `src-tauri/src/main.rs` | Add `read_file_text` command |
| `src/tauri.ts` | Add `readFileText` wrapper |
| `src/App.tsx` | Async load diff content + tree entries from Tauri |
| `src/components/DiffPane.tsx` | Accept nullable `fileDiff` + loading state |
| `src/sampleData.ts` | Keep as fallback for Vite dev mode |

## Tests to Add

- `readFileText` Rust unit test (valid file, missing file, truncation)
- `tauri.ts` mock test (verify invoke is called with right args)
- `App.tsx` loading/error state tests
- `isTauri` detection test

## Order of Implementation

1. **Phase 1 (1-2 hours):** `read_file_text` command + wire diff content
2. **Phase 2 (30 min):** Wire `scan_tree` for live file tree
3. **Phase 3 (1-2 hours):** File open dialogs + drag and drop
4. **Phase 4 (future):** Decision on Rust-format vs. pierre-format

## Risk: running the Tauri app

Currently the app is tested in `npm run dev` (Vite only). Running `cargo tauri dev`
will build the Rust backend and open a native window. This has not been tested
on this machine yet and may surface packaging issues (missing system libraries,
icon assets, code signing).

Action: try `cargo tauri dev` as the first step of Phase 1 to catch any
environment issues before writing new code.
