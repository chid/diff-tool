//! VCR-style HTTP request recording and offline replay for LLM backends.
//!
//! Captures live HTTP exchanges (request URI, sanitized headers, payload, response status,
//! response body) to a JSON cassette on disk, and replays them offline for fast, deterministic,
//! zero-cost testing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::types::CassetteMode;
use crate::{AiError, Result};

/// A recorded VCR cassette containing HTTP interactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cassette {
    pub version: u32,
    pub interactions: Vec<HttpInteraction>,
}

impl Default for Cassette {
    fn default() -> Self {
        Self {
            version: 1,
            interactions: Vec::new(),
        }
    }
}

/// A single recorded HTTP request and response interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInteraction {
    pub request: RecordedRequest,
    pub response: RecordedResponse,
    pub recorded_at: String,
}

/// Recorded details of an outgoing HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRequest {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

/// Recorded details of an incoming HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Manages loading, matching, recording, and persisting a cassette on disk.
#[derive(Debug)]
pub struct CassetteRunner {
    path: PathBuf,
    mode: CassetteMode,
    cassette: Cassette,
    dirty: bool,
}

impl CassetteRunner {
    /// Load an existing cassette or initialize a new one.
    pub fn new(path: PathBuf, mode: CassetteMode) -> Result<Self> {
        let is_replay = match mode {
            CassetteMode::Off => false,
            CassetteMode::Record => false,
            CassetteMode::Replay => true,
            CassetteMode::Auto => path.exists(),
        };

        let cassette = if path.exists() {
            let data = std::fs::read_to_string(&path)
                .map_err(|e| AiError::Cassette(format!("failed to read cassette from {}: {e}", path.display())))?;
            serde_json::from_str(&data)
                .map_err(|e| AiError::Cassette(format!("failed to parse cassette JSON from {}: {e}", path.display())))?
        } else {
            if is_replay {
                return Err(AiError::Cassette(format!(
                    "Cassette replay file not found: {}",
                    path.display()
                )));
            }
            Cassette::default()
        };

        Ok(Self {
            path,
            mode,
            cassette,
            dirty: false,
        })
    }

    /// Whether this runner should replay from the cassette rather than hitting the live network.
    pub fn is_replay(&self) -> bool {
        match self.mode {
            CassetteMode::Off => false,
            CassetteMode::Record => false,
            CassetteMode::Replay => true,
            CassetteMode::Auto => self.path.exists() && !self.cassette.interactions.is_empty(),
        }
    }

    /// Find a matching recorded response in the cassette.
    pub fn find_response(&self, method: &str, uri: &str, body: &serde_json::Value) -> Option<RecordedResponse> {
        self.cassette.interactions.iter().find_map(|interaction| {
            if interaction.request.method.eq_ignore_ascii_case(method)
                && (interaction.request.uri == uri || uris_match(&interaction.request.uri, uri))
                && interaction.request.body == *body
            {
                Some(interaction.response.clone())
            } else {
                None
            }
        })
    }

    /// Record an interaction and immediately persist the cassette to disk.
    pub fn record(
        &mut self,
        method: String,
        uri: String,
        headers: HashMap<String, String>,
        body: serde_json::Value,
        response: RecordedResponse,
    ) -> Result<()> {
        let sanitized_headers = sanitize_headers(headers);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.cassette.interactions.push(HttpInteraction {
            request: RecordedRequest {
                method,
                uri,
                headers: sanitized_headers,
                body,
            },
            response,
            recorded_at: format!("{now}"),
        });

        self.dirty = true;
        self.save()
    }

    /// Persist the cassette JSON to disk.
    pub fn save(&mut self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AiError::Cassette(format!("failed to create cassette dir: {e}")))?;
        }

        let serialized = serde_json::to_string_pretty(&self.cassette)
            .map_err(|e| AiError::Cassette(format!("failed to serialize cassette: {e}")))?;

        std::fs::write(&self.path, serialized)
            .map_err(|e| AiError::Cassette(format!("failed to write cassette to {}: {e}", self.path.display())))?;

        self.dirty = false;
        Ok(())
    }

    /// Number of recorded interactions.
    pub fn interaction_count(&self) -> usize {
        self.cassette.interactions.len()
    }
}

/// Reusable helper to execute an HTTP request through the cassette recorder/replayer.
pub async fn execute_or_record(
    cassette_runner: &Arc<Mutex<CassetteRunner>>,
    request_builder: reqwest::RequestBuilder,
    method: &str,
    uri: &str,
    headers: HashMap<String, String>,
    body: &serde_json::Value,
) -> Result<(u16, String)> {
    // 1. Check if we should replay
    let should_replay = {
        let runner = cassette_runner
            .lock()
            .map_err(|_| AiError::Cassette("cassette mutex poisoned".into()))?;
        runner.is_replay()
    };

    if should_replay {
        let runner = cassette_runner
            .lock()
            .map_err(|_| AiError::Cassette("cassette mutex poisoned".into()))?;

        if let Some(recorded) = runner.find_response(method, uri, body) {
            return Ok((recorded.status, recorded.body));
        }

        return Err(AiError::Cassette(format!(
            "No matching recorded interaction found in cassette for {method} {uri}. Body: {body}"
        )));
    }

    // 2. Perform live request
    let resp = request_builder.send().await.map_err(AiError::Http)?;
    let status = resp.status().as_u16();

    let mut resp_headers = HashMap::new();
    for (k, v) in resp.headers() {
        if let Ok(v_str) = v.to_str() {
            resp_headers.insert(k.as_str().to_string(), v_str.to_string());
        }
    }

    let text = resp.text().await.map_err(AiError::Http)?;

    // 3. Record interaction into cassette
    {
        let mut runner = cassette_runner
            .lock()
            .map_err(|_| AiError::Cassette("cassette mutex poisoned".into()))?;

        runner.record(
            method.to_string(),
            uri.to_string(),
            headers,
            body.clone(),
            RecordedResponse {
                status,
                headers: resp_headers,
                body: text.clone(),
            },
        )?;
    }

    Ok((status, text))
}

fn sanitize_headers(mut headers: HashMap<String, String>) -> HashMap<String, String> {
    for (k, v) in headers.iter_mut() {
        let lower = k.to_lowercase();
        if lower == "authorization" {
            *v = "Bearer [REDACTED]".to_string();
        } else if lower == "x-api-key" || lower == "api-key" {
            *v = "[REDACTED]".to_string();
        }
    }
    headers
}

fn uris_match(recorded: &str, incoming: &str) -> bool {
    if recorded == incoming {
        return true;
    }
    // Allow matching path suffix when host/port or /v1 prefix might vary
    if let (Some(rec_path), Some(inc_path)) = (extract_path(recorded), extract_path(incoming)) {
        let norm_rec = rec_path.strip_prefix("/v1").unwrap_or(rec_path);
        let norm_inc = inc_path.strip_prefix("/v1").unwrap_or(inc_path);
        norm_rec == norm_inc
    } else {
        false
    }
}

fn extract_path(uri: &str) -> Option<&str> {
    if let Some(idx) = uri.find("://") {
        let after_scheme = &uri[idx + 3..];
        after_scheme.find('/').map(|p| &after_scheme[p..])
    } else {
        Some(uri)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer sk-or-secret-token".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("x-api-key".to_string(), "secret-anthropic-key".to_string());

        let sanitized = sanitize_headers(headers);
        assert_eq!(sanitized.get("Authorization"), Some(&"Bearer [REDACTED]".to_string()));
        assert_eq!(sanitized.get("x-api-key"), Some(&"[REDACTED]".to_string()));
        assert_eq!(sanitized.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_cassette_record_and_replay_cycle() {
        let dir = std::env::temp_dir().join(format!("cassette-test-{}", std::process::id()));
        let cassette_path = dir.join("test_cassette.json");
        let _ = std::fs::remove_dir_all(&dir);

        // Record
        let mut runner = CassetteRunner::new(cassette_path.clone(), CassetteMode::Record).unwrap();
        assert!(!runner.is_replay());

        let req_body = serde_json::json!({ "model": "test-model", "messages": [] });
        runner
            .record(
                "POST".into(),
                "https://openrouter.ai/api/v1/chat/completions".into(),
                HashMap::new(),
                req_body.clone(),
                RecordedResponse {
                    status: 200,
                    headers: HashMap::new(),
                    body: "{\"content\":\"hello from cassette\"}".into(),
                },
            )
            .unwrap();

        assert_eq!(runner.interaction_count(), 1);
        assert!(cassette_path.exists());

        // Replay
        let replay_runner = CassetteRunner::new(cassette_path, CassetteMode::Replay).unwrap();
        assert!(replay_runner.is_replay());

        let response = replay_runner
            .find_response("POST", "https://openrouter.ai/api/v1/chat/completions", &req_body)
            .expect("should find response");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "{\"content\":\"hello from cassette\"}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
