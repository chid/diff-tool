//! AI adapter crate — LLM backend abstraction for diff-tool.
//!
//! Provides a unified `LlmBackend` trait with concrete implementations for
//! OpenAI, Anthropic, and OpenAI-compatible custom endpoints.  All API calls
//! originate from the Rust backend; API keys are never exposed to the UI.
//!
//! # Architecture
//!
//! ```text
//! LlmBackend (trait)
//! ├── OpenAiBackend   — native OpenAI Chat Completions API
//! ├── AnthropicBackend — Anthropic Messages API adapter
//! └── CustomBackend    — OpenAI-compatible endpoint (Ollama, vLLM, etc.)
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use ai_adapter::{AiAdapter, BackendConfig, Provider, ChatRequest, ChatMessage};
//!
//! let config = BackendConfig {
//!     provider: Provider::OpenAI,
//!     api_key: "sk-...".into(),
//!     model: "gpt-4o-mini".into(),
//!     ..Default::default()
//! };
//! let adapter = AiAdapter::new(config)?;
//!
//! let request = ChatRequest {
//!     model: "gpt-4o-mini".into(),
//!     messages: vec![ChatMessage::user("Hello!")],
//!     max_tokens: Some(256),
//!     temperature: Some(0.3),
//!     stream: None,
//! };
//! let response = adapter.complete(request).await?;
//! println!("{}", response.content);
//! ```

mod anthropic;
mod cache;
mod openai;
pub mod prompts;
pub mod types;

pub use anthropic::AnthropicBackend;
pub use openai::OpenAiBackend;

pub use types::{
    AiChangeSummary, AiCommitMessage, AiFullReviewReport, AiHunkExplanation,
    AiMergeResolution, BackendConfig, ChatMessage, ChatRequest, ChatResponse,
    Chunk, ConflictHunkResolution, FileCluster, FileRiskAssessment,
    PromptCategory, PromptPackageMetadata, Provider, ReviewQuestion,
    RiskLevel, Role, Usage,
};

use async_trait::async_trait;

/// Error type for AI adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: status={status}, body={body}")]
    Api { status: u16, body: String },

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("missing API key")]
    MissingApiKey,

    #[error("unsupported provider for operation: {0:?}")]
    UnsupportedProvider(Provider),

    #[error("stream error: {0}")]
    Stream(String),

    #[error("cache error: {0}")]
    Cache(String),
}

/// Result type alias for AI adapter operations.
pub type Result<T> = std::result::Result<T, AiError>;

// ── LlmBackend trait ─────────────────────────────────────────────────────

/// Abstraction over an LLM provider's HTTP API.
///
/// Implementations translate the provider-specific wire format into the
/// crate's internal `ChatRequest` / `ChatResponse` / `Chunk` types.
#[async_trait]
pub trait LlmBackend: Send + Sync + std::fmt::Debug {
    /// Send a chat completion request and wait for the full response.
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Send a chat completion request and stream response chunks.
    ///
    /// The backend pushes `Chunk::ContentDelta(...)` for each token, then a
    /// final `Chunk::Done(...)` on success or `Chunk::Error(...)` on failure.
    async fn complete_stream(
        &self,
        request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<Chunk>,
    ) -> Result<()>;
}

// ── AiAdapter — public entry point ────────────────────────────────────────

/// High-level AI adapter that delegates to the correct backend based on
/// provider configuration.
#[derive(Debug)]
pub struct AiAdapter {
    backend: Box<dyn LlmBackend>,
    config: BackendConfig,
}

impl AiAdapter {
    /// Create a new `AiAdapter` from a `BackendConfig`.
    ///
    /// Returns an error if the API key is missing or the provider is
    /// unsupported.
    pub fn new(config: BackendConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(AiError::MissingApiKey);
        }

        let reqwest_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()?;

        let backend: Box<dyn LlmBackend> = match config.provider {
            Provider::OpenAI => Box::new(openai::OpenAiBackend::new(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
                config.max_tokens,
                config.temperature,
                reqwest_client,
            )),
            Provider::Anthropic => Box::new(anthropic::AnthropicBackend::new(
                config.api_key.clone(),
                config.model.clone(),
                config.base_url.clone(),
                config.max_tokens,
                config.temperature,
                reqwest_client,
            )),
            Provider::OpenRouter => {
                let base_url = config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                let model = if config.model.is_empty() {
                    "anthropic/claude-3.5-sonnet".to_string()
                } else {
                    config.model.clone()
                };
                Box::new(openai::OpenAiBackend::new(
                    config.api_key.clone(),
                    model,
                    Some(base_url),
                    config.max_tokens,
                    config.temperature,
                    reqwest_client,
                ))
            }
            Provider::Custom => {
                let base_url = config.base_url.as_deref().ok_or_else(|| {
                    AiError::Config("Custom provider requires base_url".into())
                })?;
                Box::new(openai::OpenAiBackend::new(
                    config.api_key.clone(),
                    config.model.clone(),
                    Some(base_url.to_string()),
                    config.max_tokens,
                    config.temperature,
                    reqwest_client,
                ))
            }
        };

        Ok(Self { backend, config })
    }

    /// Send a chat completion and return the full response.
    pub async fn complete(&self, mut request: ChatRequest) -> Result<ChatResponse> {
        // Apply configured defaults if the request doesn't specify them.
        if request.max_tokens.is_none() {
            request.max_tokens = Some(self.config.max_tokens);
        }
        if request.temperature.is_none() {
            request.temperature = Some(self.config.temperature);
        }
        if request.model.is_empty() {
            request.model = self.config.model.clone();
        }
        self.backend.complete(request).await
    }

    /// Stream a chat completion, sending chunks via the provided channel.
    pub async fn complete_stream(
        &self,
        mut request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<Chunk>,
    ) -> Result<()> {
        if request.max_tokens.is_none() {
            request.max_tokens = Some(self.config.max_tokens);
        }
        if request.temperature.is_none() {
            request.temperature = Some(self.config.temperature);
        }
        if request.model.is_empty() {
            request.model = self.config.model.clone();
        }
        self.backend.complete_stream(request, tx).await
    }

    /// Get a reference to the underlying configuration.
    pub fn config(&self) -> &BackendConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_returns_error() {
        let config = BackendConfig {
            api_key: String::new(),
            ..Default::default()
        };
        let result = AiAdapter::new(config);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiError::MissingApiKey => {}
            other => panic!("expected MissingApiKey, got {other}"),
        }
    }

    #[test]
    fn custom_provider_requires_base_url() {
        let config = BackendConfig {
            provider: Provider::Custom,
            api_key: "sk-test".into(),
            base_url: None,
            ..Default::default()
        };
        let result = AiAdapter::new(config);
        assert!(result.is_err());
        match result.unwrap_err() {
            AiError::Config(msg) => assert!(msg.contains("base_url")),
            other => panic!("expected Config error, got {other}"),
        }
    }

    #[test]
    fn custom_provider_with_base_url_succeeds() {
        let config = BackendConfig {
            provider: Provider::Custom,
            api_key: "sk-test".into(),
            base_url: Some("http://localhost:11434/v1".into()),
            ..Default::default()
        };
        // This creates a reqwest client — should succeed even without a
        // server because we only validate config, not connectivity.
        let result = AiAdapter::new(config);
        assert!(result.is_ok(), "Custom provider with base_url should be valid");
    }
}
