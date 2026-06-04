//! Wiremock-based integration tests for OpenAI and Anthropic backends.
//!
//! These tests verify end-to-end HTTP request/response parsing without
//! requiring real API keys or network access.

use ai_adapter::{
    AiError, AnthropicBackend, ChatMessage, ChatRequest, ChatResponse, Chunk, LlmBackend,
    OpenAiBackend,
};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── Helpers ───────────────────────────────────────────────────────────────

fn make_request() -> ChatRequest {
    ChatRequest {
        model: "test-model".into(),
        messages: vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("Hello"),
        ],
        max_tokens: Some(256),
        temperature: Some(0.3),
        stream: None,
    }
}

fn make_streaming_request() -> ChatRequest {
    ChatRequest {
        stream: Some(true),
        ..make_request()
    }
}

fn reqwest_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Build an SSE body from data payloads. Each payload string starts on a
/// fresh line after an initial newline, matching the pattern the backends
/// expect when reading from a byte stream.
fn build_sse_body(events: &[&str]) -> String {
    let mut body = String::new();
    for event in events {
        body.push('\n');
        body.push_str(event);
        body.push('\n');
    }
    body.push('\n');
    body
}

// ── OpenAI non-streaming tests ────────────────────────────────────────────

#[tokio::test]
async fn openai_complete_nonstreaming_parses_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer sk-test"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "Hello! How can I help?"
                    }
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })))
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let response = backend.complete(make_request()).await.unwrap();

    assert_eq!(response.content, "Hello! How can I help?");
    let usage = response.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 5);
    assert_eq!(usage.total_tokens, 15);
}

#[tokio::test]
async fn openai_complete_sends_correct_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(|req: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            body["model"] == "test-model"
                && body["max_tokens"] == 256
                && body["messages"][0]["role"] == "system"
                && body["messages"][0]["content"] == "You are helpful."
                && body["messages"][1]["role"] == "user"
                && body["messages"][1]["content"] == "Hello"
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{"message": {"content": "ok"}}]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    backend.complete(make_request()).await.unwrap();
}

#[tokio::test]
async fn openai_complete_returns_api_error_on_non_200() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-bad-key".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let result = backend.complete(make_request()).await;
    match result.unwrap_err() {
        AiError::Api { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("Invalid API key"));
        }
        other => panic!("expected Api error, got {other}"),
    }
}

#[tokio::test]
async fn openai_complete_handles_empty_choices() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [],
            "usage": null
        })))
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let response = backend.complete(make_request()).await.unwrap();
    assert_eq!(response.content, "");
    assert!(response.usage.is_none());
}

#[tokio::test]
async fn openai_complete_connection_refused_returns_http_error() {
    // Point at a port where nothing is listening.
    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some("http://127.0.0.1:1".into()),
        1024,
        0.3,
        reqwest_client(),
    );

    let result = backend.complete(make_request()).await;
    assert!(result.is_err(), "expected error for connection refused");
}

// ── OpenAI streaming tests ────────────────────────────────────────────────

#[tokio::test]
async fn openai_complete_stream_parses_sse_chunks() {
    let mock_server = MockServer::start().await;

    let sse_body = build_sse_body(&[
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}",
        "data: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}",
        "data: {\"choices\":[{\"delta\":{\"content\":\"!\"},\"finish_reason\":\"stop\"}]}",
        "data: [DONE]",
    ]);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("Content-Type", "text/event-stream"),
        )
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    backend
        .complete_stream(make_streaming_request(), tx)
        .await
        .unwrap();

    let mut content = String::new();
    let mut got_done = false;

    while let Some(chunk) = rx.recv().await {
        match chunk {
            Chunk::ContentDelta(text) => content.push_str(&text),
            Chunk::Done(response) => {
                got_done = true;
                assert_eq!(response.content, "Hello world!");
                assert!(response.usage.is_none());
            }
            Chunk::Error(e) => panic!("unexpected error chunk: {e}"),
        }
    }

    assert!(got_done, "expected Done chunk at end of stream");
    assert_eq!(content, "Hello world!");
}

#[tokio::test]
async fn openai_complete_stream_handles_parse_errors() {
    let mock_server = MockServer::start().await;

    let sse_body = build_sse_body(&[
        "data: {\"choices\":[{\"delta\":{\"content\":\"good\"},\"finish_reason\":null}]}",
        "data: {\"not_a_valid_chunk\": true}",
        "data: [DONE]",
    ]);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("Content-Type", "text/event-stream"),
        )
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    backend
        .complete_stream(make_streaming_request(), tx)
        .await
        .unwrap();

    let mut error_count = 0;
    while let Some(chunk) = rx.recv().await {
        if let Chunk::Error(_) = chunk {
            error_count += 1;
        }
    }

    assert_eq!(error_count, 1, "should get one parse error for the bad line");
}

#[tokio::test]
async fn openai_complete_stream_returns_error_on_non_200() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Rate limited"))
        .mount(&mock_server)
        .await;

    let backend = OpenAiBackend::new(
        "sk-test".into(),
        "test-model".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, _rx) = tokio::sync::mpsc::channel(8);

    let result = backend
        .complete_stream(make_streaming_request(), tx)
        .await;

    match result.unwrap_err() {
        AiError::Api { status, body } => {
            assert_eq!(status, 429);
            assert!(body.contains("Rate limited"));
        }
        other => panic!("expected Api error, got {other}"),
    }
}

// ── Anthropic non-streaming tests ─────────────────────────────────────────

#[tokio::test]
async fn anthropic_complete_nonstreaming_parses_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "sk-ant-test"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [
                {"type": "text", "text": "Hello! How can I help?"},
                {"type": "text", "text": " More text."}
            ],
            "usage": {
                "input_tokens": 12,
                "output_tokens": 7
            }
        })))
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let response = backend.complete(make_request()).await.unwrap();

    assert_eq!(response.content, "Hello! How can I help? More text.");
    let usage = response.usage.unwrap();
    assert_eq!(usage.prompt_tokens, 12);
    assert_eq!(usage.completion_tokens, 7);
    assert_eq!(usage.total_tokens, 19);
}

#[tokio::test]
async fn anthropic_complete_splits_system_prompt_in_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(|req: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            body["model"] == "test-model"
                && body["max_tokens"] == 256
                && body["system"] == "You are helpful."
                && body["messages"][0]["role"] == "user"
                && body["messages"][0]["content"][0]["type"] == "text"
                && body["messages"][0]["content"][0]["text"] == "Hello"
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{"type": "text", "text": "ok"}]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    backend.complete(make_request()).await.unwrap();
}

#[tokio::test]
async fn anthropic_complete_no_system_prompt_omits_system_field() {
    let mock_server = MockServer::start().await;

    let expected_body = serde_json::json!({
        "model": "test-model",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "Hello"}]}
        ]
    });

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{"type": "text", "text": "ok"}]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let request = ChatRequest {
        model: "test-model".into(),
        messages: vec![ChatMessage::user("Hello")],
        max_tokens: None,
        temperature: None,
        stream: None,
    };

    backend.complete(request).await.unwrap();
}

#[tokio::test]
async fn anthropic_complete_returns_api_error_on_non_200() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-bad".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let result = backend.complete(make_request()).await;
    match result.unwrap_err() {
        AiError::Api { status, body } => {
            assert_eq!(status, 403);
            assert!(body.contains("Forbidden"));
        }
        other => panic!("expected Api error, got {other}"),
    }
}

#[tokio::test]
async fn anthropic_complete_handles_empty_content() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [],
            "usage": {
                "input_tokens": 5,
                "output_tokens": 0
            }
        })))
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let response = backend.complete(make_request()).await.unwrap();
    assert_eq!(response.content, "");
    assert!(response.usage.is_some());
}

#[tokio::test]
async fn anthropic_complete_connection_refused_returns_error() {
    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some("http://127.0.0.1:1".into()),
        1024,
        0.3,
        reqwest_client(),
    );

    let result = backend.complete(make_request()).await;
    assert!(result.is_err(), "expected error for connection refused");
}

// ── Anthropic streaming tests ─────────────────────────────────────────────

#[tokio::test]
async fn anthropic_complete_stream_parses_content_block_delta() {
    let mock_server = MockServer::start().await;

    let sse_body = "\
\nevent: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\" world\"}}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
";

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("Content-Type", "text/event-stream"),
        )
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    backend
        .complete_stream(make_streaming_request(), tx)
        .await
        .unwrap();

    let mut content_deltas: Vec<String> = Vec::new();
    let mut done_response: Option<ChatResponse> = None;

    while let Some(chunk) = rx.recv().await {
        match chunk {
            Chunk::ContentDelta(text) => content_deltas.push(text),
            Chunk::Done(resp) => done_response = Some(resp),
            Chunk::Error(e) => panic!("unexpected error chunk: {e}"),
        }
    }

    assert_eq!(content_deltas, vec!["Hello", " world"]);
    let done = done_response.expect("expected Done chunk");
    assert_eq!(done.content, "Hello world");
    let usage = done.usage.unwrap();
    assert_eq!(usage.completion_tokens, 2);
}

#[tokio::test]
async fn anthropic_complete_stream_ignores_non_delta_events() {
    let mock_server = MockServer::start().await;

    let sse_body = "\
\nevent: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_123\"}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"A\"}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
";

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(sse_body)
                .insert_header("Content-Type", "text/event-stream"),
        )
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    backend
        .complete_stream(make_streaming_request(), tx)
        .await
        .unwrap();

    let mut content = String::new();
    while let Some(chunk) = rx.recv().await {
        match chunk {
            Chunk::ContentDelta(text) => content.push_str(&text),
            Chunk::Done(_) => {}
            Chunk::Error(e) => panic!("unexpected error: {e}"),
        }
    }

    assert_eq!(content, "A");
}

#[tokio::test]
async fn anthropic_complete_stream_returns_error_on_non_200() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal error"))
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        1024,
        0.3,
        reqwest_client(),
    );

    let (tx, _rx) = tokio::sync::mpsc::channel(8);

    let result = backend
        .complete_stream(make_streaming_request(), tx)
        .await;

    match result.unwrap_err() {
        AiError::Api { status, body } => {
            assert_eq!(status, 500);
            assert!(body.contains("Internal error"));
        }
        other => panic!("expected Api error, got {other}"),
    }
}

// ── Max tokens fallback test ──────────────────────────────────────────────

#[tokio::test]
async fn anthropic_uses_max_tokens_fallback() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(|req: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();
            body["model"] == "test-model"
                && body["max_tokens"] == 4096
                && body["system"] == "You are helpful."
                && body["messages"][0]["role"] == "user"
                && body["messages"][0]["content"][0]["type"] == "text"
                && body["messages"][0]["content"][0]["text"] == "Hello"
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": [{"type": "text", "text": "ok"}]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let backend = AnthropicBackend::new(
        "sk-ant-test".into(),
        "claude-test".into(),
        Some(mock_server.uri()),
        4096,
        0.3,
        reqwest_client(),
    );

    let request = ChatRequest {
        model: "test-model".into(),
        messages: vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("Hello"),
        ],
        max_tokens: None,
        temperature: Some(0.3),
        stream: None,
    };

    backend.complete(request).await.unwrap();
}
