# CLAUDE.md

Guidance for working in this repository.

## Project shape

Rust 2024 Cargo workspace. Crates:

- `diff-core` — deterministic text/AST diff engine (no I/O policy surprises: always normalizes CRLF/CR → LF in memory).
- `git-adapter` — filesystem/tree scanning.
- `office-adapter` — Word/Excel/PowerPoint structural diffing.
- `ai-adapter` — LLM backends (Anthropic, OpenAI) with caching.
- `diff-cli` — CLI front end (`clap`).
- `apps/desktop/src-tauri` — Tauri desktop shell.

## Commands

Run from the workspace root; these cover all crates.

```bash
cargo build                       # build everything
cargo test                        # run all unit + integration tests
cargo test -p diff-core           # one crate
cargo fmt --all                   # format (run before committing)
cargo clippy --all-targets --all-features -- -D warnings   # lint; warnings are errors
```

Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` and confirm they pass before claiming work is done.

## Rust best practices for this repo

### Workspace hygiene
- Shared dependencies and package metadata live in the root `Cargo.toml` under
  `[workspace.dependencies]` / `[workspace.package]`. New crates inherit with
  `version.workspace = true`, `edition.workspace = true`, and
  `some-dep.workspace = true`. Don't pin a second, divergent version of a shared dep in a crate.
- Keep crate boundaries honest: `diff-core` stays pure (no network, no global state) so its output is reproducible. Push I/O and side effects into the adapter/CLI crates.

### Error handling
- Each crate defines its own `thiserror`-derived error enum (e.g. `DiffError`,
  `AdapterError`, `OfficeError`, `AiError`). Follow the existing pattern:
  structured variants carrying context (`{ path, source }`), not stringly-typed
  errors. Use `#[source]` / `#[from]` so error chains stay intact.
- Return `Result<T, ThisCrateError>` from public APIs. Never paper over a failure
  by returning a default or empty value.
- **No `unwrap()` / `expect()` / `panic!` in library code paths.** They're fine in
  tests, `build.rs`, and genuinely-impossible cases — and when used for an
  invariant, prefer `expect("why this cannot fail")` with a message explaining the
  invariant, not a bare `unwrap()`. (There's existing debt here; don't add to it.)

### Unsafe & mmap
- `unsafe` requires a justification comment stating the invariant being upheld.
- `memmap2::Mmap` is `unsafe` to create because the mapped file can change under
  us. Treat mapped bytes as a read-only snapshot; if a caller might truncate/modify
  the file concurrently, that risk must be documented at the call site.

### API & documentation
- Keep the crate-level `//!` docs current — `diff-core`'s line-ending policy doc is
  the model: state opinionated behavior and *why*. Document every `pub` item.
- Prefer borrowing (`&str`, `&[u8]`, `&Path`) in signatures; take owned values only
  when you must store them. Accept `impl AsRef<Path>` / `IntoIterator` where it
  reads naturally.
- Don't leak third-party types (e.g. `imara_diff`, `tree_sitter`) through public
  signatures unless intentional — wrap them so the dependency stays swappable.

### Determinism & performance
- Diff output must be deterministic. When using `rayon` or any concurrency, collect
  and sort/merge results into a stable order before returning; never let thread
  scheduling affect output.
- Reach for `rayon` only on measured hot paths. There are bench tests
  (`*_bench.rs`, `approaches_bench.rs`) — add or update a benchmark when changing an
  algorithm rather than guessing.

### Testing
- Unit tests live in `#[cfg(test)] mod tests`; cross-cutting/fixture tests live in
  each crate's `tests/`. Fixture-driven cases (`fixture_cases.rs`) are the norm for
  diff behavior — add a fixture rather than a one-off assertion when adding a case.
- Use `tempfile` for tests that touch the filesystem; never write into the repo tree.

### Recommended (not yet configured)
- Add a workspace `[workspace.lints]` table (`rust` + `clippy`) to enforce these
  centrally — e.g. `unwrap_used`, `expect_used` as `warn`, and
  `clippy::all` / `clippy::pedantic` selectively.
- Add a `rust-toolchain.toml` to pin the toolchain so everyone builds 2024-edition
  consistently.
