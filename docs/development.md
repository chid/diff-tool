# Development Guide

## Setup

```bash
npm_config_cache=/tmp/npm-cache npm install
```

The temp npm cache is only needed on machines where `~/.npm` is not writable.

## Running The App

```bash
npm run dev
```

This starts the Vite app at `http://127.0.0.1:5173/`.

For Tauri development:

```bash
npm run tauri -- dev
```

## Running The CLI

```bash
cargo run -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt
cargo run -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt --json
cargo run -p diff-cli -- dir examples/folder-basic/left --json
```

## Adding A Diff Fixture

1. Create `fixtures/cases/<case-name>/`.
2. Add `left.txt`, `right.txt`, and `case.json`.
3. Set `ignore` to `none`, `whitespace`, or `whitespace-and-case`.
4. Set expected additions, deletions, unchanged lines, and truncation behavior.
5. Run `cargo test -p diff-core`.

## Adding A CLI Behavior

1. Implement the command behavior in `crates/diff-cli`.
2. Add or update tests in `crates/diff-cli/tests`.
3. Prefer JSON assertions for machine-readable output.
4. Document any human output that scripts may rely on.

## Adding A Desktop Feature

1. Add or update shared TypeScript types in `apps/desktop/src/types.ts`.
2. Add Tauri command helpers in `apps/desktop/src/tauri.ts`.
3. Keep presentational components separate from IPC calls.
4. Add visual or interaction tests when the UI test harness exists.

## Generated Artifacts

Do not commit `target/`, `dist/`, `node_modules/`, or generated perf fixtures.

The existing Tauri `gen/schemas` files are tracked because they are part of the
current scaffold and help document the app capability surface.
