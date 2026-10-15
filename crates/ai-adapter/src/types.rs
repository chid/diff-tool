//! Shared types for the AI adapter crate.
//!
//! Defines `ChatRequest`, `ChatResponse`, `Chunk` (for streaming), and
//! feature-specific request/response shapes used by prompt templates.

use serde::{Deserialize, Serialize};

// ── Core chat types ──────────────────────────────────────────────────────

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// A request to an LLM backend.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// A complete (non-streaming) response from an LLM backend.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ChatResponse {
    pub content: String,
    /// Token usage if reported by the provider.
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

/// A streaming response chunk from an LLM backend.
#[derive(Debug, Clone, PartialEq)]
pub enum Chunk {
    /// A content delta (partial response text).
    ContentDelta(String),
    /// The streaming response completed successfully.
    Done(ChatResponse),
    /// An error occurred mid-stream.
    Error(String),
}

// ── Backend configuration ─────────────────────────────────────────────────

/// Provider selection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
    /// OpenRouter aggregator (OpenAI-compatible API with https://openrouter.ai/api/v1).
    OpenRouter,
    /// An OpenAI-compatible endpoint (Ollama, llama.cpp, vLLM, LM Studio, etc.).
    Custom,
}

/// Configuration for building an `LlmBackend`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
    /// Base URL override (required for Custom, optional for OpenAI/Anthropic).
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Timeout in seconds for HTTP requests.
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// Optional cassette recording/replay configuration.
    #[serde(default)]
    pub cassette: Option<CassetteConfig>,
}

fn default_max_tokens() -> u32 { 1024 }
fn default_temperature() -> f32 { 0.3 }
fn default_timeout_secs() -> u64 { 60 }

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            provider: Provider::OpenAI,
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
            base_url: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            timeout_secs: default_timeout_secs(),
            cassette: None,
        }
    }
}

impl BackendConfig {
    /// Configure cassette recording or replay for this backend.
    pub fn with_cassette(mut self, path: impl Into<std::path::PathBuf>, mode: CassetteMode) -> Self {
        self.cassette = Some(CassetteConfig {
            path: path.into(),
            mode,
        });
        self
    }
}

/// Cassette operational mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CassetteMode {
    /// Normal direct HTTP execution without recording or replaying.
    Off,
    /// Record live HTTP requests and responses into the cassette file.
    Record,
    /// Replay requests from the cassette file without any network calls.
    Replay,
    /// If the cassette file exists, replay it; if not, record it.
    Auto,
}

/// Cassette file and mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CassetteConfig {
    pub path: std::path::PathBuf,
    pub mode: CassetteMode,
}

// ── Feature-specific types ────────────────────────────────────────────────

/// AI-generated change summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiChangeSummary {
    /// One-sentence summary of the change.
    pub summary: String,
    /// Bullet-point list of intents.
    pub intents: Vec<String>,
    /// Notable risk signals (empty if none).
    pub risks: Vec<String>,
}

/// AI-generated merge conflict resolution for a single hunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConflictHunkResolution {
    /// The resolved text for this hunk.
    pub resolved_text: String,
    /// Explanation of why this resolution was chosen.
    pub explanation: String,
    /// Whether the AI is uncertain about this resolution.
    pub ambiguous: bool,
}

/// AI-generated merge resolution for an entire file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiMergeResolution {
    /// The fully resolved file content.
    pub resolved_file: String,
    /// Per-hunk resolutions and explanations.
    pub hunks: Vec<ConflictHunkResolution>,
}

/// Risk assessment for a single file change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileRiskAssessment {
    pub risk_level: RiskLevel,
    /// Short justification for the risk level.
    pub justification: String,
    /// Files, modules, or concepts potentially affected.
    pub affected_areas: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    #[serde(alias = "Low", alias = "LOW")]
    Low,
    #[serde(alias = "Medium", alias = "MEDIUM")]
    Medium,
    #[serde(alias = "High", alias = "HIGH")]
    High,
}

/// A cluster of related file changes with a theme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileCluster {
    pub theme: String,
    pub description: String,
    pub file_paths: Vec<String>,
}

/// Review question generated by the AI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewQuestion {
    pub question: String,
    /// Optional file or line reference the question is about.
    pub context: Option<String>,
}

/// AI-generated commit message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiCommitMessage {
    pub subject: String,
    pub body: Option<String>,
}

/// Category of a prompt package.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptCategory {
    Review,
    Audit,
    Authoring,
    Merge,
    Explain,
}

/// Metadata describing a prompt package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptPackageMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PromptCategory,
    pub version: u32,
    pub default_temperature: f32,
    pub default_max_tokens: u32,
    pub tags: Vec<String>,
}

/// A comprehensive full review report combining summary, risk, and reviewer questions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiFullReviewReport {
    pub summary: AiChangeSummary,
    pub risk: FileRiskAssessment,
    pub questions: Vec<ReviewQuestion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_commit: Option<AiCommitMessage>,
}

/// Plain-English explanation of a hunk or change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiHunkExplanation {
    pub explanation: String,
    pub key_points: Vec<String>,
}

