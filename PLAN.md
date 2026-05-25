  # PLAN.md: MVP Desktop Ship Plan

  ## Summary

  Ship target: MVP Desktop. The first release should be a credible local desktop/CLI preview for text file comparison and single-root
  folder scanning, with clear limits. Office comparison is parked as experimental and must not block the first ship target or appear in
  release claims.

  Current review findings from this checkout:

  - npm run check passes.
  - CARGO_HOME=/tmp/cargo-home cargo test -p office-adapter passes, but with many warnings.
  - INSTA_UPDATE=no CARGO_HOME=/tmp/cargo-home cargo test --workspace fails because diff-core reads Office fixture case.json files as
    text-diff fixtures.
  - npm run perf:quick runs, but the perf harness treats dt diff exit code 1 as failure even though 1 now correctly means “differences
    found.”

  ## Key Changes

  - Add root PLAN.md with this MVP plan, then keep TODO.md as the tactical backlog and update stale checked-off items.
  - Stabilize the test layout: either move Office fixtures out of fixtures/cases or teach text fixture discovery to only load text-diff
    schemas.
  - Fix the perf harness to treat dt diff exit codes 0 and 1 as successful executions, parse JSON for both, and reserve failure for
    errors/timeouts.
  - Make Rust DiffResult the MVP desktop rendering contract. Use the native renderer for shipped text diffs; keep @pierre/diffs only as
    fallback/spike code until a renderer ADR decides otherwise.
  - Finish the desktop open flow: no hardcoded README.md/TODO.md defaults in Tauri mode; support left/right file dialogs, folder picker
    for scan, loading/error/empty states, and binary-file errors from Rust.
  - Keep MVP folder support scoped to single-root scan, not true left/right folder compare. Label UI and docs accordingly.
  - Clean CLI release behavior: keep documented dt diff exit codes; make nonexistent dt dir roots error instead of silently returning
    success; remove, hide, or make unimplemented merge and git install return a non-success “not implemented” error.
  - Park Office work: keep office-adapter green if it remains in the workspace, clean warning noise, and document it as experimental
    with no desktop/CLI public surface for MVP.
  - Prepare packaging: commit intentional Tauri capabilities/icons, set bundle config intentionally, produce a macOS local preview
    build, and only claim Windows/Linux after smoke validation.

  ## Test Plan

  Required green checks before release:

  - npm run check
  - INSTA_UPDATE=no CARGO_HOME=/tmp/cargo-home cargo test --workspace
  - npm run build
  - npm run test:e2e:cli
  - npm run test:e2e after making Playwright headless-by-default and ensuring tests write only under target/e2e
  - npm run perf:quick after the exit-code fix
  - npm run tauri -- build for the desktop artifact

  Coverage to add or fix:

  - Text fixture discovery ignores Office schemas.
  - CLI tests cover missing dt dir root and unimplemented command behavior.
  - Desktop tests cover file-open compare, Rust-backed renderer output, binary/unreadable input errors, and settings persistence.
  - E2E CLI-backed render test should not create transient files under apps/desktop/public; use Playwright routing or target/e2e
    instead.

  ## Assumptions

  - MVP release means a local developer/early-user preview, not a signed/notarized public commercial release.
  - Office, merge resolution, true left/right folder comparison, Git status, image diff, and AI features are post-MVP.
  - Generated artifacts such as target/, test-results/, handoff files, and .bak files should not be committed.
  - Windows work should use the existing SSH helper scripts rather than complex inline PowerShell.
