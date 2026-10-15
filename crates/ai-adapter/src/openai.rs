//! OpenAI Chat Completions API backend.
//!
//! Implements `LlmBackend` for the native OpenAI API (`/v1/chat/completions`).
//! Also works with any OpenAI-compatible endpoint (Ollama, vLLM, LM Studio, etc.)
//! by setting `base_url` to the custom endpoint.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::{ChatRequest, ChatResponse, Chunk, Role, Usage};
use crate::{AiError, LlmBackend, Result};

/// Default OpenAI API base URL.
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::cassette::CassetteRunner;

#[derive(Debug)]
pub struct OpenAiBackend {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    cassette: Option<Arc<Mutex<CassetteRunner>>>,
}

impl OpenAiBackend {
    pub fn new(
        api_key: String,
        _model: String,
        base_url: Option<String>,
        _max_tokens: u32,
        _temperature: f32,
        client: reqwest::Client,
    ) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            client,
            cassette: None,
        }
    }

    pub fn with_cassette(mut self, cassette: Option<Arc<Mutex<CassetteRunner>>>) -> Self {
        self.cassette = cassette;
        self
    }

    fn url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    fn build_body(&self, request: &ChatRequest) -> serde_json::Value {
        let messages: Vec<OpenAiMessage> = request
            .messages
            .iter()
            .map(|m| OpenAiMessage {
                role: role_to_str(m.role),
                content: m.content.clone(),
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
        });

        if let Some(mt) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(mt);
        }
        if let Some(t) = request.temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(s) = request.stream {
            body["stream"] = serde_json::json!(s);
        }

        body
    }
}

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

// ── OpenAI wire types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

// ── Streaming wire types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    #[serde(default)]
    content: String,
}

// ── LlmBackend impl ───────────────────────────────────────────────────────

#[async_trait]
impl LlmBackend for OpenAiBackend {
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse> {
        let body = self.build_body(&request);
        let url = self.url();

        let (status, text) = if let Some(cassette) = &self.cassette {
            let req_builder = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body);

            let mut headers = HashMap::new();
            headers.insert("Content-Type".into(), "application/json".into());
            headers.insert("Authorization".into(), format!("Bearer {}", self.api_key));

            crate::cassette::execute_or_record(
                cassette,
                req_builder,
                "POST",
                &url,
                headers,
                &body,
            )
            .await?
        } else {
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            let status = resp.status().as_u16();
            let text = resp.text().await.map_err(AiError::Http)?;
            (status, text)
        };

        if status < 200 || status >= 300 {
            return Err(AiError::Api {
                status,
                body: text,
            });
        }

        let openai_resp: OpenAiResponse = serde_json::from_str(&text)
            .map_err(|e| AiError::Parse(format!("failed to parse OpenAI response: {e}")))?;

        let content = openai_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            usage: openai_resp.usage,
        })
    }

    async fn complete_stream(
        &self,
        request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<Chunk>,
    ) -> Result<()> {
        if let Some(cassette) = &self.cassette {
            let is_replay = cassette.lock().map(|r| r.is_replay()).unwrap_or(false);
            if is_replay {
                let full = self.complete(request).await?;
                let _ = tx.send(Chunk::ContentDelta(full.content.clone())).await;
                let _ = tx.send(Chunk::Done(full)).await;
                return Ok(());
            }
        }

        let mut body = self.build_body(&request);
        body["stream"] = serde_json::json!(true);

        let resp = self
            .client
            .post(&self.url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = tx
                .send(Chunk::Error(format!("API error {}: {}", status.as_u16(), body)))
                .await;
            return Err(AiError::Api {
                status: status.as_u16(),
                body,
            });
        }

        use futures::StreamExt;

        let mut stream = resp.bytes_stream();
        let mut full_content = String::new();

        while let Some(chunk_result) = stream.next().await {
            let bytes = match chunk_result {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx
                        .send(Chunk::Error(format!("stream read error: {e}")))
                        .await;
                    return Err(AiError::Stream(e.to_string()));
                }
            };

            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };

                match serde_json::from_str::<OpenAiStreamChunk>(data) {
                    Ok(chunk) => {
                        if let Some(choice) = chunk.choices.into_iter().next() {
                            if !choice.delta.content.is_empty() {
                                full_content.push_str(&choice.delta.content);
                                if tx
                                    .send(Chunk::ContentDelta(choice.delta.content))
                                    .await
                                    .is_err()
                                {
                                    // Receiver dropped — stop streaming.
                                    return Ok(());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Chunk::Error(format!(
                                "failed to parse stream chunk: {e}"
                            )))
                            .await;
                    }
                }
            }
        }

        let _ = tx
            .send(Chunk::Done(ChatResponse {
                content: full_content,
                usage: None,
            }))
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatMessage;

    #[test]
    fn build_body_includes_messages_and_model() {
        let backend = OpenAiBackend::new(
            "sk-test".into(),
            "gpt-4o-mini".into(),
            None,
            1024,
            0.3,
            reqwest::Client::new(),
        );

        let request = ChatRequest {
            model: "gpt-4o-mini".into(),
            messages: vec![ChatMessage::user("Hello")],
            max_tokens: Some(256),
            temperature: Some(0.5),
            stream: None,
        };

        let body = backend.build_body(&request);
        assert_eq!(body["model"], "gpt-4o-mini");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"], "Hello");
        assert_eq!(body["max_tokens"], 256);
        assert_eq!(body["temperature"], 0.5);
    }

    #[test]
    fn build_body_defaults_no_stream() {
        let backend = OpenAiBackend::new(
            "sk-test".into(),
            "gpt-3.5-turbo".into(),
            None,
            512,
            0.0,
            reqwest::Client::new(),
        );

        let request = ChatRequest {
            model: "gpt-3.5-turbo".into(),
            messages: vec![ChatMessage::system("You are helpful.")],
            max_tokens: None,
            temperature: None,
            stream: None,
        };

        let body = backend.build_body(&request);
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("stream").is_none());
    }

    #[test]
    fn custom_base_url_strips_trailing_slash() {
        let backend = OpenAiBackend::new(
            "sk-test".into(),
            "model".into(),
            Some("http://localhost:11434/v1/".into()),
            1024,
            0.3,
            reqwest::Client::new(),
        );

        assert_eq!(
            backend.url(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn role_to_str_maps_all_variants() {
        assert_eq!(role_to_str(Role::System), "system");
        assert_eq!(role_to_str(Role::User), "user");
        assert_eq!(role_to_str(Role::Assistant), "assistant");
    }
}
