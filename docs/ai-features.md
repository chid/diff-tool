# AI Feature Ideation Catalog

## Summary

Add AI-powered features to the diff/merge tool using a BYOK (Bring Your Own
Key) model with OpenAI/Anthropic-compatible backends. All AI features are
optional — the core diff engine works fully offline without any API key.

The architecture is defined in [ADR 0002](decisions/0002-ai-features-byok.md).

## Product USP

No desktop diff/merge tool has shipped meaningful AI features. Kaleidoscope,
Beyond Compare, Araxis, DeltaWalker, and P4Merge are all AI-free. Meanwhile,
web-based tools like CodeRabbit, GitHub Copilot, and SemanticDiff have
proven demand for:

- natural-language change summaries
- intent/risk analysis for diffs
- AI-assisted merge conflict resolution
- review question generation
- impact analysis across a changed file set

A desktop diff tool with local-first AI (user-controlled keys, optional
offline mode, no SaaS tax) can differentiate by combining the privacy and
speed of a native app with AI enrichment that users opt into on their own
terms.

## Guiding Principles

1. **Zero AI in the default path.** Every AI feature must have a clear
   non-AI fallback or disabled state. The diff tool must be fully functional
   without an API key.
2. **BYOK only.** The app never bundles inference cost. Users configure their
   own provider and key. No data passes through a proxy controlled by this
   project.
3. **Rust backend, not UI.** All LLM calls originate from the Rust backend
   via `reqwest`. API keys are stored in the OS keychain. The React UI never
   touches API keys or raw LLM responses.
4. **Deterministic core, probabilistic enrichment.** Diff computation, merge
   base selection, folder scans, etc. remain deterministic and testable. AI
   features are a presentation layer on top of the same deterministic data.
5. **Streaming by default.** Long-running AI responses (summaries, conflict
   resolutions) stream tokens into the UI so the user sees progress.

## Feature Catalog

### P0: AI Change Summary

**What:** For a given file diff or folder diff, generate a concise
natural-language summary of what changed and why.

**User story:** A developer opens a diff of `src/auth/login.tsx` and sees:
"Extracted the OAuth callback into a `useOAuth` hook, added loading state and
error boundary. No behavior change to the login flow itself."

**Implementation:**

- Rust sends the unified diff text + file extension to the LLM.
- Prompt asks for: (1) one-sentence summary, (2) bullet-point intent list,
  (3) notable risk signals.
- Response is streamed into a collapsible "AI Summary" panel above or beside
  the diff view.
- Result is cached keyed by `(file_hash, model_name)`. Cache lives in
  `~/.cache/dt/ai-summaries/`.

**UI:** A small wand icon button in the toolbar / file header. Clicking it
expands or generates the summary. The panel shows a streaming response with
a "Regenerate" button.

**CLI:** `dt explain <diff-file> [--provider anthropic] [--model claude-3-haiku]`

**Fail state:** "AI summary requires an API key. Configure one in Settings."

---

### P0: AI-Assisted Merge Conflict Resolution

**What:** For a conflicted file (with `<<<<<<<` / `=======` / `>>>>>>>`
markers), suggest a resolved version that considers intent from both sides.

**User story:** A developer hits a merge conflict in `api/routes.go`. Instead
of manually picking ours/theirs or editing the raw markers, they click
"Resolve with AI" and see a suggested resolution with an explanation of how
each conflict hunk was resolved.

**Implementation:**

- Rust parses the conflicted file into conflict hunks using `diff-core`'s
  merge model (planned in P4).
- For each hunk, send the base/ours/theirs sections to the LLM.
- Prompt asks the model to reason through each conflict, explaining the
  decision.
- Return a suggested merged file and a per-hunk explanation.
- User can accept all, accept per-hunk, or edit manually.

**UI:** A "Resolve with AI" button in the merge view header. Suggested
resolutions shown inline with accept/reject per hunk. Explanations in a side
panel or tooltip.

**CLI:** `dt merge --ai left.file right.file base.file`

**Caching:** Prompt+result cached by `(file_hash, conflict_hash, model)`. If
the same conflict is seen again (e.g., reopened), return cached result.

**Fail state:** The model may produce invalid code or remove critical logic.
The AI should explicitly flag hunks where the correct resolution is ambiguous
(see CodeRabbit's approach). Build/lint validation runs after AI resolution
to catch syntax errors.

---

### P1: Risk And Impact Scoring

**What:** For each changed file in a folder diff or PR, assign a risk score
(low/medium/high) and list potential impacts on other parts of the system.

**User story:** A reviewer sees a folder diff with 12 changed files. The
tool highlights "high risk: `src/core/scheduler.rs` — changes to the task
queue may affect `src/worker/pool.rs` and `src/api/jobs.rs`."

**Implementation:**

- Rust sends the diff for each file to the LLM with a structured prompt.
- Prompt asks for: risk level, affected areas (files, modules, concepts),
  and a short justification.
- Risk JSON is returned and merged into the `DiffResult` or
  `FolderDiffResult` model.
- The file tree pane shows risk badges (red/orange/green dots) next to each
  file.

**CLI:** `dt dir --risk`

**Caching:** Same hash-keyed cache as summaries.

**Fail state:** Risk scoring is best-effort. Never block a merge or hide a
file based on AI risk scores. Always show the raw diff alongside the AI
assessment.

---

### P1: Review Question Generation

**What:** For a given diff, generate 3-5 questions the reviewer should ask
the author. These highlight edge cases, missing tests, security concerns,
and architectural mismatches.

**User story:** A reviewer opens a diff and sees: "Things to ask the author:
(1) What happens when `token` is null? (2) Is there a test for the rate
limiter edge case? (3) Does this change affect the WebSocket reconnection
path?"

**Implementation:**

- Same LLM call as the summary but with an additional "questions" output in
  the response schema.
- Questions are displayed in a collapsible section below the summary.
- Each question can be clicked to scroll to the relevant diff lines (requires
  line-anchor metadata from the LLM, which is approximate).

**UI:** A "Review Questions" accordion below the AI summary. Each question
is a clickable card.

**Fail state:** Generate generic or unhelpful questions. Acceptable for v1
— users will ignore low-quality questions. Improve prompts over time.

---

### P1: AI Commit Message Generation

**What:** Given a staged diff, generate a conventional commit message.

**User story:** A developer runs `dt commit-message` or clicks "Generate" in
the commit panel. The tool outputs:

```
feat(auth): extract OAuth callback into useOAuth hook

- Added loading state and error boundary to OAuth flow
- No behavior change to existing login flow
- Extracted callback logic from LoginPage to custom hook
```

**Implementation:**

- Rust sends the unified diff to the LLM with a conventional commits prompt.
- Returns a subject line and optional body.
- User can edit before committing.

**CLI:** `dt commit-msg` (reads staged diff from `git diff --cached` or
accepts a diff file)

---

### P2: Intent Clustering For Folder Diffs

**What:** Group changed files by the intent or theme of the change, not by
directory path. For example, a PR that touches 20 files might be clustered
as: "Auth refactor (8 files)", "API migration (6 files)", "Dependency
upgrades (4 files)", "Lint fixes (2 files)".

**User story:** A reviewer sees a large folder diff organized by intent
clusters instead of a flat file tree. They can expand/cluster collapse by
cluster to focus on related changes.

**Implementation:**

- Rust sends the concatenated diffs (or file-level summaries) to the LLM.
- LLM returns a list of clusters with file paths and a short description.
- The React tree pane switches to cluster view when AI clustering is
  enabled.
- Users can also define custom clusters by file globs (non-AI).

**CLI:** `dt dir --cluster`

**Caching:** Cached by `(dir_hash, model_name)`.

---

### P2: Agent-Review Workspace

**What:** Compare the output of multiple AI coding agents working on the same
task. Show a unified diff of their diffs — where they agree, where they
diverge, and where one approach looks risky.

**User story:** A developer runs the same refactoring prompt through Claude
Code and Copilot. They import both agent output branches into the diff tool
and see a 3-way comparison: "These 4 files are identical in both agents.
Agent A changed `utils.ts` differently than Agent B — here's why each
approach is different."

**Implementation:**

- Import two branches (or diff results) and compute a diff-of-diffs.
- AI explains the divergence: "Agent A inlined the helper, Agent B kept it
  separate. Agent A's approach reduces call overhead but makes testing
  harder."
- Display as a 3-panel view: Agent A | Unified | Agent B, with AI callouts.

**Note:** This is already in the backlog as P5. The AI layer makes it
substantially more useful by explaining divergences rather than just showing
them.

---

### P3: AI-Driven Diff Triage Ranking

**What:** Rank changed files by review priority — "review these first" vs.
"these are safe." Uses a combination of: change complexity, dependency
impact, test coverage gaps, and AI risk assessment.

**User story:** A developer with a 50-file PR sees a ranked list: "Files
requiring careful review (4)", "Routine changes (12)", "Low-risk (34)."

**Implementation:**

- Combine deterministic metrics (file size, churn, dependency count, test
  coverage) with AI risk scores.
- Output a ranked file list with badges.
- All ranking logic lives in Rust. The AI call is optional: without AI,
  ranking uses only deterministic metrics.

**CLI:** `dt dir --rank`

---

### P3: Explain Line / Explain Hunk

**What:** Right-click on any diff line or hunk and ask "Explain this
change." Get a focused, one-paragraph explanation of what that specific
change does and why it might have been made.

**User story:** A reviewer sees a suspicious line in a diff. They right-click
→ "Explain this line." The AI responds: "This line adds a fallback to an
empty string when `user.name` is null. Previously the code would crash here
with a null pointer exception."

**Implementation:**

- Send the hunk context (5-10 lines around the selected line) to the LLM.
- Show the explanation in a popover or side panel.
- For speed, use a cheap model (GPT-4o-mini, Claude Haiku) by default.

**UI:** Context menu item "Explain this change" in the diff pane. Result
appears in a floating tooltip or side panel.

---

### P3: Evidence-Linked Summaries

**What:** AI-generated summaries that cite specific line numbers or file
offsets as evidence. For example: "Extracted OAuth callback to
`useOAuth` hook (lines 45-78). Added loading state (line 92)."

**Implementation:**

- Prompt the LLM to include line references in its output.
- Render references as clickable links that scroll the diff pane to the
  relevant line.
- This requires the LLM to produce accurate line numbers, which is
  notoriously unreliable. Mitigations:
  - Post-process the LLM output to verify line numbers against the actual
    diff.
  - If verification fails, show the text without clickable links.
  - Prompt with the exact line-numbered diff text to improve accuracy.

---

## Architecture

### Rust Backend (new crate: `crates/ai-adapter/`)

```text
crates/ai-adapter/
  Cargo.toml
  src/
    lib.rs              # LlmBackend trait, public API
    openai.rs           # OpenAI-compatible backend
    anthropic.rs        # Anthropic Messages API adapter
    prompts.rs          # Prompt templates for each feature
    cache.rs            # Hash-keyed result cache
    types.rs            # ChatRequest, ChatResponse, feature-specific types
```

The trait:

```rust
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, req: ChatRequest) -> Result<ChatResponse>;
    async fn complete_stream(
        &self,
        req: ChatRequest,
        tx: tokio::sync::mpsc::Sender<Chunk>,
    ) -> Result<()>;
}
```

### Tauri IPC Commands

All AI features are exposed as Tauri commands on the Rust side:

```rust
#[tauri::command]
async fn ai_summarize_diff(diff_text: String, lang: String) -> Result<StreamHandle>;

#[tauri::command]
async fn ai_resolve_conflict(file_path: String) -> Result<AiResolution>;

#[tauri::command]
async fn ai_rank_files(diff_json: Value) -> Result<Vec<RankedFile>>;

#[tauri::command]
async fn ai_cluster_files(diff_json: Value) -> Result<Vec<FileCluster>>;

#[tauri::command]
async fn ai_explain_hunk(hunk_text: String, lang: String) -> Result<String>;
```

### CLI

```bash
dt explain <diff-file>                      # AI change summary
dt explain <diff-file> --hunk <line>        # Explain specific hunk
dt merge --ai <ours> <theirs> <base>        # AI merge resolution
dt commit-msg [--staged]                    # Generate commit message
dt dir --risk <path>                        # Risk scoring
dt dir --cluster <path>                     # Intent clustering
dt dir --rank <path>                        # Triage ranking
```

### Settings

Added to the existing settings registry (`settingsTypes.ts`):

| Key | Type | Default | Description |
|---|---|---|---|
| ai.provider | select | openai | `openai`, `anthropic`, `custom` |
| ai.model | string | gpt-4o-mini | Model name |
| ai.baseUrl | string | https://api.openai.com/v1 | For custom endpoints |
| ai.apiKey | (keychain) | — | Stored in OS keychain |
| ai.maxTokens | number | 1024 | Max response tokens |
| ai.temperature | float | 0.3 | Lower = more deterministic |
| ai.enabled | toggle | false | Master toggle for AI features |
| ai.cacheEnabled | toggle | true | Cache AI results to disk |

### Security

- API keys are stored in the OS keychain via the `keyring` crate, never in
  settings JSON, `localStorage`, or source files.
- The frontend never receives the API key. All LLM calls go through the Rust
  backend.
- Users can disable AI entirely at the setting level.
- When AI is enabled, a subtle indicator in the toolbar shows which provider
  and model are active.

### Caching Strategy

- Cache key = `SHA256(diff_text + model_name + prompt_template_version)`.
- Cache location = `~/.cache/dt/ai-cache/` (or `$XDG_CACHE_HOME`).
- Cache entry TTL = 7 days by default, configurable.
- Cache is invalidated when the prompt template version changes (bump
  `PROMPT_VERSION` const).
- User can clear the cache from settings.

### Offline / Fail States

Every AI feature has three states:

1. **AI available** — API key configured and reachable. Feature works.
2. **AI not configured** — No API key. Feature shows a one-line message:
   "Enable AI in Settings (requires an API key)."
3. **AI error** — API call failed (network, auth, rate limit). Feature shows:
   "AI temporarily unavailable. [Retry] [Settings]"

No AI feature ever blocks the non-AI workflow. The diff tool works fully
offline. AI features are purely additive enrichment.

## Implementation Sequence

### Phase 1: Backend Foundation

- [ ] Create `crates/ai-adapter/` with `LlmBackend` trait.
- [ ] Implement `OpenAiBackend` (native OpenAI API).
- [ ] Implement `AnthropicBackend` (adapter layer).
- [ ] Add `reqwest` with streaming support.
- [ ] Add `keyring` crate for OS keychain storage.
- [ ] Add API key management Tauri commands (store, retrieve, validate,
      delete).
- [ ] Add cache implementation with hash-keyed file storage.
- [ ] Write unit tests for all three backends with mock HTTP.
- [ ] Integration test with real API (opt-in, gated by env var).

### Phase 2: AI Change Summary

- [ ] Add `prompts::summary` prompt template.
- [ ] Add `ai_summarize_diff` Tauri command with streaming.
- [ ] Add `<AiSummaryPanel>` React component.
- [ ] Add "Explain" button to file header and toolbar.
- [ ] Add `dt explain` CLI command.
- [ ] Wire cache for summary results.
- [ ] Add settings UI for AI provider/model/key.

### Phase 3: AI Merge Conflict Resolution

- [ ] Ensure `diff-core` can parse conflict markers into structured hunks.
- [ ] Add `prompts::resolve_conflict` prompt template.
- [ ] Add `ai_resolve_conflict` Tauri command.
- [ ] Add merge resolution UI with accept/reject per hunk.
- [ ] Add build/lint validation step after AI resolution.
- [ ] Add `dt merge --ai` CLI flag.

### Phase 4: Risk, Clustering, Triage

- [ ] Add risk scoring prompt and Tauri command.
- [ ] Add intent clustering prompt and Tauri command.
- [ ] Add triage ranking combining deterministic + AI signals.
- [ ] Add risk badges to file tree pane.
- [ ] Add cluster view toggle to file tree pane.
- [ ] Add `dt dir --risk`, `dt dir --cluster`, `dt dir --rank`.

### Phase 5: Explain Line, Review Questions, Commit Messages

- [ ] Add explain-hunk prompt and context menu.
- [ ] Add review-questions prompt and accordion UI.
- [ ] Add commit-message prompt and `dt commit-msg`.
- [ ] Add evidence-linked summaries with line-number verification.

### Phase 6: Agent-Review Workspace

- [ ] Add diff-of-diffs computation in Rust.
- [ ] Add 3-panel agent comparison view.
- [ ] Add divergence explanation prompt.
