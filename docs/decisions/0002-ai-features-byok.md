# ADR 0002: AI Features — BYOK with OpenAI/Anthropic Compat

## Status

Accepted as the default strategy for AI feature development.

## Context

The diff/merge desktop market has no incumbent with meaningful AI features.
Kaleidoscope, Beyond Compare, Araxis Merge, DeltaWalker, and P4Merge are all
AI-free. Meanwhile, AI code review tools (CodeRabbit, Qodo, GitHub Copilot) and
semantic diff tools (SemanticDiff, Kenaz semantic-diff) have proven that AI
adds real value: change summaries, risk scoring, merge conflict resolution, and
intent explanations.

Shipping AI features in a local-first desktop tool raises several questions:

- Who pays for LLM inference? Per-user SaaS pricing conflicts with the
  local-first product identity.
- Which model provider? Locking to one vendor creates downstream risk if
  pricing, quality, or availability changes.
- How private is the code? Users must trust that their diffs are not used for
  training or stored on third-party servers.
- What about offline use? Local-first means some features should degrade
  gracefully without a network.

## Decision

### Bring Your Own Key (BYOK) is the default

Users configure an API key for their preferred provider in the app settings.
The app never bundles, resells, or proxies API access. This means:

- No inference cost liability for the project.
- Users choose their provider, model, and data-handling policy.
- The app stays local-first; all diff computation still runs locally. Only
  AI-specific enrichment calls the configured API.
- Offline: AI features show a clear "API key required" or "offline" state
  instead of failing silently.

### OpenAI / Anthropic compat is the final integration layer

The AI backend speaks the OpenAI Chat Completions API format. Anthropic's
Messages API is mapped through a thin adapter. The abstraction boundary is
a single Rust trait:

```rust
#[async_trait]
trait LlmBackend {
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse>;
}
```

Concrete implementations:

- `OpenAiBackend` — native OpenAI API calls.
- `AnthropicBackend` — translates Anthropic Messages API to the internal
  `ChatRequest`/`ChatResponse` shape.
- Any OpenAI-compatible local server (Ollama, vLLM, llama.cpp, LM Studio) works
  by pointing the OpenAI backend at a custom `base_url`.

This means a user can run `ollama run codellama` on their machine, configure
`base_url: http://localhost:11434/v1` in app settings, and get AI features
with zero data leaving their computer.

### API calls happen from the Rust backend, not the UI

The Tauri Rust side owns all HTTP requests to LLM providers. The React UI
never holds API keys. This means:

- API keys are stored in the OS keychain (via `keyring` crate), not in
  `localStorage` or settings JSON.
- Requests are not interceptable from DevTools.
- The Rust backend can stream responses, enforce rate limits, and implement
  retry/backoff without involving the UI layer.
- The same AI backend code is shared by CLI commands and the desktop app.

## Consequences

- Users must bring their own API key. This is a friction point, but it aligns
  with the local-first identity and avoids per-user SaaS costs. Document it
  clearly in onboarding.
- Model quality is not controlled by the app. A user with a weak model will
  get weak summaries. The settings UI should show model recommendations and
  minimum capability hints.
- The Rust backend needs an HTTP client with streaming support (`reqwest` with
  `json` and `stream` features). This is a new dependency for the backend.
- API key management requires OS keychain integration. The `keyring` crate
  works on macOS (Keychain), Linux (Secret Service), and Windows (Credential
  Manager), but needs platform-specific testing.
- Ollama/local model support is free but comes with the caveat that smaller
  models produce lower-quality output. The design should handle incomplete,
  malformed, or empty LLM responses gracefully.
- The trait abstraction makes it straightforward to add new providers (Google
  Gemini, Grok, DeepSeek, etc.) without changing call sites.

## Alternatives Considered

### Bundled inference (ONNX / llama.cpp)

Running a small model locally without any API call would be the purest
local-first approach. Rejected for now because:

- Adding 1-8 GB of model weights to the app bundle conflicts with the current
  lean scaffold.
- Small models produce noticeably worse summaries and conflict resolutions.
- The performance characteristics on low-end hardware are unknown.
- BYOK can later be extended with a "local model" option that points at a
  running Ollama instance — same architecture, zero bundling cost.

### SaaS proxy (central API with per-user billing)

Having the project act as a proxy and charge for inference would generate
revenue but conflicts with the local-first product identity. The BYOK approach
is more honest about what the app is: a local diff tool that optionally uses
the user's chosen AI provider.

### UI-layer-only AI (JavaScript fetch from React)

Simpler to implement, but rejected because API keys would be visible in
DevTools, bundle, and source maps. The Rust backend approach is more secure
and enables CLI reuse.

## Future Considerations

- Add a `dt ai explain` CLI command that takes a diff file and prints an
  LLM-generated summary to stdout.
- Add prompt caching and result caching (keyed by diff hash) so repeated views
  of the same diff do not re-query the API.
- Add a "local model" option that detects Ollama on localhost and offers it
  as a first-class provider choice.
- If bundled inference becomes practical (e.g., a 1 GB model matches GPT-4o
  quality), add it as a fourth backend without changing the trait.
