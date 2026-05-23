# Agent Work Packages

## Agent A: Market And Spec Research

Update the competitor matrix for Kaleidoscope, Sublime Merge, Beyond Compare,
Araxis, DeltaWalker, P4Merge, Fork, Tower, GitKraken, SmartGit,
`@pierre/diffs`, and `@pierre/trees`.

Deliver licensing, maintenance, platform, price, and feature notes. Separate
competitors from implementation dependencies.

## Agent B: Pierre UI Spike

Own `apps/desktop`. Validate `@pierre/diffs/react` and `@pierre/trees/react`
behind the local wrapper components. Record any Shadow DOM styling, bundle,
worker, Tauri packaging, or API stability issues.

## Agent C: Rust Diff Core

Own `crates/diff-core`. Expand deterministic line diffs, intraline spans,
whitespace modes, context folding, binary detection, encodings, and golden
fixtures.

## Agent D: Git And Filesystem Backend

Own `crates/git-adapter`. Implement repository discovery, status, changed-file
lists, folder scan events, ignore rules, rename detection, conflict discovery,
and safe Git write-through via installed `git`.

## Agent E: CLI And Integration

Own `crates/diff-cli`. Implement `dt diff`, `dt dir`, `dt merge`, `dt git
install`, JSON output, GUI launch argument passing, and Git exit-status
behavior.

## Agent F: Test Harness And Benchmarks

Own `fixtures`, `scripts`, and `benches`. Import license-compatible fixtures
from Git, GNU diffutils, libgit2, diff-match-patch/dissimilar, and tree-sitter
tests. Add large-repo benchmarks for Rust, CPython, LLVM, and Linux checkouts.

## Agent G: AI Features

Own `crates/ai-adapter` and the AI enrichment layer across CLI and desktop.
Implement the `LlmBackend` trait with OpenAI/Anthropic-compatible backends, API
key management via OS keychain, and hash-keyed result caching. Build prompt
templates for change summaries, merge conflict resolution, risk scoring, intent
clustering, review questions, commit messages, explain-hunk, and agent-review
workspace. Every AI feature must have a clear offline/disabled fallback.

See [ADR 0002](docs/decisions/0002-ai-features-byok.md) and
[docs/ai-features.md](docs/ai-features.md).
