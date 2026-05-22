# Diff Tool

Greenfield prototype for a cross-platform, local-first diff and merge tool.

The workspace has three tracks:

- Rust core crates for deterministic diff, folder, Git, and CLI behavior.
- Tauri + React desktop shell for the product surface.
- Research and agent briefs for splitting implementation work across agents.

## Commands

```bash
npm_config_cache=/tmp/npm-cache npm install
npm run dev
npm run test
cargo run -p diff-cli -- diff README.md README.md --json
```

Use `npm_config_cache=/tmp/npm-cache` on this machine until the local npm cache
ownership problem in `~/.npm` is repaired.
