# MCP Interface Plan

## Feasibility

Yes. This diff tool can provide an MCP interface cleanly because its useful
operations already sit behind serializable Rust models:

- `diff_core::DiffRequest` -> `diff_core::DiffResult`
- `git_adapter::scan_tree()` -> `Vec<git_adapter::DirEntryDiff>`
- `git_adapter::compare_folders()` -> `git_adapter::FolderDiff`
- `office_adapter::compare_office_files()` -> `office_adapter::OfficeDiffResult`

MCP should be an additional headless integration surface, not a desktop feature.
The server should run as a local stdio process first, reuse the existing Rust
crates, and return the same deterministic JSON shapes that the CLI and Tauri
app use. A remote HTTP MCP transport can come later if there is a real product
need for a shared diff service.

Relevant current MCP references:

- MCP architecture: https://modelcontextprotocol.io/docs/learn/architecture
- MCP specification: https://modelcontextprotocol.io/specification/2025-06-18/basic/index
- Official Rust SDK: https://github.com/modelcontextprotocol/rust-sdk

## Product Goal

Let coding agents and AI desktop clients ask the diff tool for local,
deterministic comparison context without shelling out to ad hoc commands or
parsing human output.

Target workflows:

1. Compare two files and receive structured hunks, stats, warnings, and
   binary/UTF-8 errors.
2. Scan a directory tree and receive deterministic file entries.
3. Compare two folders and classify paths as added, deleted, modified,
   unchanged, or directory.
4. Ask for stable JSON schemas and prompts that teach an agent how to summarize
   or review the returned diff data.
5. Later: expose Office diff and Git-aware comparison once those surfaces are
   stable enough to be treated as agent-facing contracts.

## Protocol Shape

MCP exposes three relevant primitive types:

- Tools: executable operations such as file diffing and folder scanning.
- Resources: read-only context such as JSON schema documents and cached result
  handles.
- Prompts: reusable prompt templates for summarizing or reviewing a diff.

The MVP should expose tools and resources. Prompts are useful but can be Phase 2.
The server should not use MCP sampling initially; it should stay deterministic
and avoid embedding any LLM provider logic in the MCP server.

## Architecture

Add a new Rust crate:

```text
crates/diff-mcp/
  Cargo.toml
  src/main.rs          # stdio server entrypoint
  src/server.rs        # MCP handler and tool registration
  src/tools.rs         # typed tool inputs and core adapters
  src/resources.rs     # schema/static resource registry
  tests/               # protocol and tool integration tests
```

Update the workspace:

```toml
members = [
  "crates/diff-core",
  "crates/git-adapter",
  "crates/diff-cli",
  "crates/office-adapter",
  "crates/ai-adapter",
  "crates/diff-mcp",
  "apps/desktop/src-tauri"
]
```

Use the official Rust MCP SDK (`rmcp`) with the `server`, `macros`,
`schemars`, and `transport-io` features. Add `tokio` only to `diff-mcp`; do not
force async dependencies into `diff-core` or `git-adapter`.

The preferred launch command should be:

```bash
cargo run -p diff-mcp
```

Optionally add a CLI alias later:

```bash
dt mcp
```

The alias is convenient for packaging, but it should delegate to the same
server code instead of duplicating MCP handling inside `diff-cli`.

## Tool Contract

### `diff_files`

Compare two local files.

Input:

```json
{
  "left": "/absolute/or/root-relative/path.txt",
  "right": "/absolute/or/root-relative/path.txt",
  "context": 3,
  "ignore": "none",
  "algorithm": "myers",
  "deadline_ms": 5000,
  "max_bytes": 10485760,
  "language": null,
  "syntax_depth": null
}
```

Output:

```json
{
  "result": {
    "files": [],
    "stats": { "additions": 0, "deletions": 0, "unchanged": 0 },
    "warnings": [],
    "truncated": false
  },
  "has_changes": false
}
```

Implementation:

- Convert input to `diff_core::DiffRequest`.
- Reuse `diff_core::compare_files()`.
- Return `DiffResult` as structured JSON, not serialized text.
- Map `DiffError::BinaryFile` and read errors into MCP tool errors with stable
  codes in the JSON body.
- Preserve syntax diff controls, but include an `"experimental"` warning when
  `algorithm = "syntax"`.

### `scan_tree`

List files below a local root.

Input:

```json
{
  "root": "/absolute/or/root-relative/path",
  "include_hidden": true
}
```

Output:

```json
{
  "entries": [
    {
      "path": "src/main.rs",
      "status": "unchanged",
      "size": 1234,
      "mtime": 1760000000,
      "hash": null,
      "children": []
    }
  ]
}
```

Implementation:

- Reuse `git_adapter::scan_tree()`.
- Add root existence and root-is-directory validation before scanning.
- Keep output sorted by relative path.

### `compare_folders`

Classify matching paths across two local roots.

Input:

```json
{
  "left_root": "/absolute/or/root-relative/left",
  "right_root": "/absolute/or/root-relative/right",
  "compare_by": "size-and-mtime",
  "include_hidden": true
}
```

Output:

```json
{
  "entries": [],
  "summary": {
    "added": 0,
    "deleted": 0,
    "modified": 0,
    "unchanged": 0,
    "directories": 0
  }
}
```

Implementation:

- Reuse `git_adapter::compare_folders()` for the MVP.
- Add `FolderDiffSummary` in `diff-mcp` first, or promote it to
  `git-adapter` if the desktop app also needs it.
- Treat content hashing as a later option; the current `CompareBy::Contents`
  model exists but is not implemented.

### `diff_folder_file`

Compare one selected relative path across two roots.

Input:

```json
{
  "left_root": "/repo/a",
  "right_root": "/repo/b",
  "path": "src/lib.rs",
  "context": 3,
  "algorithm": "myers"
}
```

Output: same as `diff_files`.

Implementation:

- Join `path` against both roots after path normalization.
- Reject absolute `path`, `..`, and symlink escapes.
- Reuse `diff_files` internals.

This tool is important because agents often first call `compare_folders`, then
need a focused diff for one modified file without reconstructing paths.

## Resource Contract

Expose static schemas and lightweight documentation:

- `diff-tool://schema/diff-result`
- `diff-tool://schema/folder-diff`
- `diff-tool://schema/office-diff`
- `diff-tool://docs/tool-contract`

The schemas should be generated from Rust types where possible with `schemars`.
If a type cannot derive a useful schema immediately, add a hand-written JSON
schema in `crates/diff-mcp/schemas/` and test that sample tool output validates
against it.

Do not expose arbitrary filesystem files as MCP resources in the MVP. File
contents already flow through explicit tools with path validation.

## Prompt Contract

Phase 2 can add these prompts:

- `summarize_diff`: asks the host model to summarize a `DiffResult`.
- `review_diff`: asks for correctness, regression, and missing-test risks.
- `explain_folder_changes`: asks for a concise explanation of a folder summary.

Prompt arguments should reference tool result JSON, not file paths. The server
should not fetch extra files implicitly while rendering a prompt.

## Path And Security Model

MCP clients can cause local file reads through tools, so the MVP must be
conservative:

1. Default allowed root is the server process working directory.
2. Accept additional roots only through explicit server startup flags, for
   example `diff-mcp --allow-root /repo --allow-root /tmp/case`.
3. Canonicalize every path before use and reject paths outside allowed roots.
4. Reject absolute child paths in `diff_folder_file`.
5. Reject `..` traversal and symlink escapes.
6. Do not execute shell commands.
7. Do not run Git commands in the MVP.
8. Do not send file contents to AI providers from the MCP server.
9. Cap file sizes with `max_bytes`, defaulting to the current 10 MiB limit.
10. Surface binary-file and invalid-UTF-8 behavior exactly as `diff-core` does.

For stdio transport, credentials should come from environment or process launch
configuration if they are ever needed. The MVP should require no credentials.

## Implementation Phases

### Phase 0: Contract cleanup

- Add or confirm public constructors/parsers for algorithm, ignore mode, and
  syntax depth so MCP does not duplicate CLI parsing rules.
- Add `compare_texts()` to `diff-core` if in-memory diffing is desired. This is
  optional for the first MCP MVP, but it will be useful for agents that already
  hold generated text.
- Decide whether `git_adapter::FolderDiff` should include a summary directly or
  whether summaries stay adapter-specific in `diff-mcp`.
- Add fixture outputs for `diff_files`, `scan_tree`, and `compare_folders`.

### Phase 1: Local stdio MCP MVP

- Create `crates/diff-mcp`.
- Add `rmcp` and `tokio` dependencies to the new crate only.
- Implement startup flags: `--allow-root`, `--max-bytes`, `--deadline-ms`.
- Implement path policy and tests.
- Implement `diff_files`, `scan_tree`, `compare_folders`, and
  `diff_folder_file`.
- Return structured JSON content and stable error payloads.
- Add integration tests that invoke the server over stdio and call each tool.
- Add a small client configuration example for MCP hosts that launch local
  servers.

Exit criteria:

- `cargo test -p diff-mcp` passes.
- A protocol smoke test can list tools and call `diff_files` against
  `examples/text-basic`.
- Tool output matches the existing Rust model shape.
- Path escape tests fail closed.

### Phase 2: Schemas, prompts, and docs

- Generate or check in JSON schemas for all exposed result types.
- Expose schema resources.
- Add prompt templates for summary, review, and folder explanation.
- Add README documentation for configuring the MCP server in local clients.
- Add `npm run check` or `./scripts/check.sh` coverage for the new crate.

Exit criteria:

- MCP resource listing includes all schema resources.
- Sample tool outputs validate against schemas.
- Prompt rendering tests are deterministic and do not read undeclared files.

### Phase 3: Packaging and CLI alias

- Add `dt mcp` if the binary packaging story is clearer with one executable.
- Add release artifact notes for `diff-mcp` as a separate binary if not.
- Add install examples for macOS, Windows, and Linux.
- Confirm the stdio server does not emit non-protocol logs on stdout.

Exit criteria:

- Packaged binary can be configured as a local MCP server.
- Logs go to stderr or MCP logging notifications only.
- Existing `dt diff`, desktop, and tests keep working unchanged.

### Phase 4: Advanced tools

Add these only after the underlying product surfaces are stable:

- `office_diff` for `.docx`, `.pptx`, and `.xlsx`, clearly marked
  experimental until Office support is no longer parked.
- `git_worktree_status` after `git-adapter` owns real Git repository discovery.
- `git_compare_refs` after commit/branch comparison APIs exist.
- `merge_preview` after the 3-way merge model exists.
- `semantic_diff` after JSON/YAML/TOML semantic diff mode exists.

## Test Plan

Unit tests:

- Path canonicalization accepts allowed roots and rejects escapes.
- Tool input parsing rejects invalid algorithms, ignore modes, and roots.
- Folder summary counts statuses correctly.
- Error mapping preserves read, binary, and invalid-argument details.

Integration tests:

- Start `diff-mcp` over stdio.
- Call MCP initialization.
- Call `tools/list` and assert the MVP tools are present.
- Call `tools/call` for `diff_files` using `examples/text-basic`.
- Call `tools/call` for `scan_tree` using `examples/folder-basic`.
- Call `tools/call` for `compare_folders` using temp fixtures.
- Assert no stdout content appears outside JSON-RPC frames.

Regression fixtures:

- Identical files.
- Add/delete/modify text files.
- Binary file error.
- Invalid UTF-8 warning.
- Syntax diff fallback warning.
- Directory root missing and file-passed-as-directory errors.

Manual verification:

```bash
cargo test -p diff-mcp
./scripts/check.sh
cargo run -p diff-mcp -- --allow-root "$PWD"
```

Use MCP Inspector or a small test client to list tools and call
`diff_files` against:

```text
examples/text-basic/left.txt
examples/text-basic/right.txt
```

## Open Decisions

1. Should the first public launch command be `cargo run -p diff-mcp`, `dt mcp`,
   or both?
2. Should `diff_text` be part of the MVP, requiring a `diff-core` in-memory
   comparison API, or should the first version stay path-only?
3. Should `compare_folders` use the current size/mtime behavior, or wait until
   content hashing is implemented?
4. Should Office diff be hidden until the Office adapter is no longer
   experimental, or exposed as an explicitly experimental tool?
5. Should schemas be generated with `schemars` from Rust types, hand-written for
   cleaner client docs, or both?

## Recommended MVP

Build Phase 1 first with these choices:

- New `crates/diff-mcp` binary.
- Stdio transport only.
- Tools: `diff_files`, `scan_tree`, `compare_folders`, `diff_folder_file`.
- Resources: defer schemas until Phase 2 unless `schemars` is trivial during
  implementation.
- Prompts: defer until Phase 2.
- Allowed roots default to process cwd, with explicit `--allow-root` overrides.
- No AI, no Git commands, no remote HTTP transport.

This gives agents a useful local diff interface quickly while preserving the
repo's existing architecture rule: backend crates stay independent from UI and
integration surfaces.
