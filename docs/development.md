# Developing and testing Diff Tool

Use the browser for the shortest path from an edit to a visible redline. Use the Rust CLI to exercise the filesystem and native comparison models. The [architecture guide](architecture.md) maps each user action to the engine and result type it uses.

## Start the browser app

Run these commands from the repository root with Node 22.18 or newer on the 22.x line. This also supports running the TypeScript benchmark directly. The frontend uses npm workspaces and a committed lockfile.

```sh
npm ci
npm run dev
```

Open the loopback URL printed by Vite and append `/?tab=documents`. The default is `http://127.0.0.1:5173/?tab=documents`. Vite can select a different port when 5173 is occupied. For a production build, `npm run build` type-checks and writes the static app, workers, and OCR assets to `apps/desktop/dist/`. Use `npm run preview` to serve that build. [Frontend scripts](../apps/desktop/package.json) · [Vite and OCR asset configuration](../apps/desktop/vite.config.ts)

## Compare the same Word pair in two runtimes

1. In **Documents**, choose `fixtures/cases/office-word-basic/left.docx` as **Original** and `right.docx` from the same directory as **Revised**. The sentence changes from “Hello from the left file.” to “Hello from the right file.” The review marks the two changed words.
2. Choose **Download tracked Word report**. In Word, accepting all changes yields the right-hand sentence. Rejecting them in a fresh copy yields the left-hand sentence. The browser regression checks both readings in the downloaded file. [Word fixtures](../fixtures/cases/office-word-basic/) · [Browser test](../tests/e2e/document_comparison.spec.ts)
3. From the repository root, run the Rust comparison below with the project Rust toolchain (the repository specifies Rust 1.95+). Its JSON summary for this fixture has one changed section and one atomic change. The command exits with `1` because the files differ. [Fixture expectation](../fixtures/cases/office-word-basic/case.json) · [CLI entry point](../crates/diff-cli/src/main.rs)

```sh
cargo run --locked -p diff-cli -- office diff \
  fixtures/cases/office-word-basic/left.docx \
  fixtures/cases/office-word-basic/right.docx --json
```

For a text comparison, run `cargo run --locked -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt --json`. The example reports two additions, one deletion, and two unchanged lines. For these comparison commands the exit codes are `0` for identical, `1` for differences, and `2` for an error.

For the desktop shell, install the platform prerequisites for Tauri 2 and run `npm --workspace apps/desktop run tauri -- dev`. Tauri starts Vite using its configured port 5173. Stop a standalone Vite instance first so the desktop loads the checkout you intend. The Documents toolbar accepts the Word pair above. For the native text engine, use the main file pickers in **Changes**. [Desktop configuration](../apps/desktop/src-tauri/tauri.conf.json)

## Where to make a change

| Symptom or feature | First place to look | What to test |
| --- | --- | --- |
| A header is missing, a slide is out of order, or a formula change is invisible | The format-specific extractor and its section keys. [Document readers](../apps/desktop/src/documents/read.ts) | Use real package parts. Change only the header, slide order, or formula so a body-text assertion cannot mask the defect. |
| Both texts are correct but the wrong words are marked | `wordSpans` and, for two Word bodies, `bodyChanges`. [Redline engine](../apps/desktop/src/documents/redline.ts) | Pin the retained words and both readings. Include repeated words, tabs, or blank lines if they trigger the case. [Alignment tests](../apps/desktop/src/documents/redline.test.ts) |
| The UI looks right but the saved DOCX reads differently | `trackedReport` and the Word XML it emits. [Report writer](../apps/desktop/src/documents/redline.ts) | Unzip the download and reconstruct accepted and rejected text, including separators. [Export regression](../apps/desktop/src/documents/redline.test.ts) |
| Revising one input shows stale results or reruns OCR | Selection handling in `App` and the cache, abort, and result checks in `DocumentReview`. [Selection](../apps/desktop/src/App.tsx) · [Review lifecycle](../apps/desktop/src/documents/DocumentReview.tsx) | Replace one side, then choose a different file with the old filename. Exercise recovery after a corrupt input. [Browser tests](../tests/e2e/document_comparison.spec.ts) |
| Only the CLI or native desktop result is wrong | Text hunks belong to `diff-core`, paths to `git-adapter`, Office changes to `office-adapter`. For a desktop-only issue, trace the request from TypeScript into its Rust handler. [IPC client](../apps/desktop/src/tauri.ts) · [Handlers](../apps/desktop/src-tauri/src/main.rs) | Add a fixture or crate test. For an IPC contract change, update both the Rust serialization and the TypeScript consumer. [Fixture conventions](../fixtures/cases/README.md) · [UI types](../apps/desktop/src/types.ts) |

## Choose a verification loop

| Check | Command from the repository root | What it exercises |
| --- | --- | --- |
| Word alignment and tracked output | `npx vitest run --root apps/desktop src/documents/redline.test.ts` | Direct comparison and exported Word XML, including move detection and whitespace round-trips. |
| Browser document workflow | `npx playwright test 'document_.*spec.ts'` | File pickers, the three document demos, drops, errors, navigation, OCR, visual PDF comparison, and downloads. On a fresh machine, run `npx playwright install chromium` first. Playwright starts or reuses Vite on port 5173. |
| Browser build | `npm run build` | TypeScript compilation and production assets. Serve the build with `npm run preview` when investigating asset-loading errors. |
| Native Office behavior | `cargo test --locked -p office-adapter` | The Rust adapter and its fixtures. For CLI output and exit codes, use `cargo test --locked -p diff-cli`. |
| Repository pre-handoff check | `./scripts/check.sh` | TypeScript checking and Rust workspace tests, including the Tauri crate. This check needs the native build dependencies. Run the relevant Vitest and Playwright checks separately. |
| Document engine performance | `node scripts/benchmark-document-redline.mjs` | Seven timed comparisons after warm-up for each of three 1,500-clause inputs. Each run verifies both text projections. For changes to the Rust text engine, use `npm run perf:quick`. |

To check agreement with Litera, download the two linked source PDFs, install Poppler's `pdftohtml`, and run `node scripts/benchmark-litera.mjs /path/to/redline.pdf` once per file. The script verifies each PDF's checksum. Report exact-line matches, matched reference words, and extra marked words. The published baseline and the line-selection rules are in the [document comparison benchmark](document-comparison.md#published-reference-benchmark).

**Reproducing the native checks.** At `969f223`, two CLI JSON snapshots contain absolute checkout paths and one records fixture modification times. On a fresh Linux checkout, those three assertions differed only in those fields. Read the snapshot diff before accepting an update. The full workspace check also builds Tauri and needs system development packages discoverable by `pkg-config`, including GDK 3 and Cairo on Linux. [CLI snapshots](../crates/diff-cli/tests/snapshots/) · [Workspace check](../scripts/check.sh)

Keep generated builds, downloads, Playwright output, and benchmark runs in ignored locations. Commit a minimal reproducible input under [fixtures/cases](../fixtures/cases/) or [the browser document fixtures](../tests/e2e/fixtures/documents/) when a change needs a durable regression.
