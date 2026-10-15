use ai_adapter::{AiAdapter, BackendConfig, ChatMessage, ChatRequest, Provider, Role};
use diff_core::{DiffAlgorithm, DiffMode, DiffRequest, IgnoreMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Emitter;

#[cfg(test)]
use tauri::{Listener, Manager};

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

/// Office document comparison request.
#[derive(Debug, Deserialize)]
struct CompareOfficeRequest {
    left: PathBuf,
    right: PathBuf,
    /// For Excel: only compare these sheets (empty = all).
    #[serde(default)]
    sheets: Vec<String>,
}

/// AI chat comparison / analysis request.
#[derive(Debug, Deserialize)]
struct AiCompareRequest {
    /// Provider: "openai", "anthropic", or "custom".
    provider: String,
    /// API key (never persisted to disk).
    api_key: String,
    /// Model name (e.g. "gpt-4o-mini", "claude-sonnet-4-20250514").
    #[serde(default)]
    model: String,
    /// Base URL override (required for custom providers).
    #[serde(default)]
    base_url: Option<String>,
    /// Maximum tokens for the completion (default 1024).
    #[serde(default = "default_ai_max_tokens")]
    max_tokens: u32,
    /// Sampling temperature 0.0–2.0 (default 0.3).
    #[serde(default = "default_ai_temperature")]
    temperature: f32,
    /// Chat messages.
    messages: Vec<AiChatMessage>,
}

fn default_ai_max_tokens() -> u32 { 1024 }
fn default_ai_temperature() -> f32 { 0.3 }

#[derive(Debug, Deserialize)]
struct AiChatMessage {
    /// "system", "user", or "assistant".
    role: String,
    content: String,
}

/// Response payload for the `compare_ai` command.
#[derive(Debug, Serialize)]
struct AiCompareResponse {
    content: String,
    /// Token usage if reported by the provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CompareFilesRequest {
    left: PathBuf,
    right: PathBuf,
    #[serde(default)]
    algorithm: Option<String>,
    #[serde(default)]
    deadline_ms: Option<u64>,
    #[serde(default)]
    progress: bool,
}

/// Payload emitted to the frontend via `diff-progress` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiffProgressPayload {
    stage: String,
    /// File path for the `reading` stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// File size in bytes for the `reading` stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
}

/// Parse an algorithm string, defaulting to Myers for unknown values.
fn parse_algorithm(s: &Option<String>) -> DiffAlgorithm {
    match s.as_deref() {
        Some("histogram") => DiffAlgorithm::Histogram,
        Some("patience") => DiffAlgorithm::Patience,
        Some("myers") | None => DiffAlgorithm::Myers,
        _ => DiffAlgorithm::Myers,
    }
}

/// Map a `diff_core::Progress` stage to a `DiffProgressPayload`.
/// Extracted so both `compare_files` and the test suite use the same mapping.
fn progress_to_payload(p: diff_core::Progress) -> DiffProgressPayload {
    match p {
        diff_core::Progress::Reading { path, bytes } => DiffProgressPayload {
            stage: "reading".to_string(),
            path: Some(path.display().to_string()),
            bytes: Some(bytes),
        },
        diff_core::Progress::Normalizing => DiffProgressPayload {
            stage: "normalizing".to_string(),
            path: None,
            bytes: None,
        },
        diff_core::Progress::ComputingDiff => DiffProgressPayload {
            stage: "computing_diff".to_string(),
            path: None,
            bytes: None,
        },
        diff_core::Progress::ComputingIntralineSpans => DiffProgressPayload {
            stage: "computing_intraline_spans".to_string(),
            path: None,
            bytes: None,
        },
        diff_core::Progress::BuildingHunks => DiffProgressPayload {
            stage: "building_hunks".to_string(),
            path: None,
            bytes: None,
        },
    }
}

#[tauri::command]
fn compare_files<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    request: CompareFilesRequest,
) -> Result<diff_core::DiffResult, String> {
    let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> =
        if request.progress {
            let app = app_handle.clone();
            Some(Box::new(move |p| {
                let _ = app.emit("diff-progress", progress_to_payload(p));
            }))
        } else {
            None
        };

    diff_core::compare_files_with_progress(
        DiffRequest {
            left: request.left,
            right: request.right,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: Some(DEFAULT_MAX_BYTES),
            algorithm: parse_algorithm(&request.algorithm),
            deadline_ms: request.deadline_ms.filter(|&ms| ms > 0),
            language: None,
            syntax_depth: None,
        },
        progress_cb,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn scan_tree(root: PathBuf) -> Result<Vec<git_adapter::DirEntryDiff>, String> {
    git_adapter::scan_tree(&root).map_err(|error| error.to_string())
}

#[tauri::command]
fn compare_folders(left: PathBuf, right: PathBuf) -> Result<git_adapter::FolderDiff, String> {
    git_adapter::compare_folders(&left, &right).map_err(|error| error.to_string())
}

#[tauri::command]
fn compare_office<R: tauri::Runtime>(
    _app_handle: tauri::AppHandle<R>,
    request: CompareOfficeRequest,
) -> Result<office_adapter::OfficeDiffResult, String> {
    let mut opts = office_adapter::OfficeDiffOptions::default();
    if !request.sheets.is_empty() {
        opts.sheets = request.sheets;
    }
    office_adapter::compare_office_files(request.left, request.right, opts)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn compare_ai(request: AiCompareRequest) -> Result<AiCompareResponse, String> {
    let provider = match request.provider.as_str() {
        "anthropic" => Provider::Anthropic,
        "custom" => Provider::Custom,
        _ => Provider::OpenAI,
    };

    let model = if request.model.is_empty() {
        default_model_for_provider(provider).to_string()
    } else {
        request.model
    };

    let config = BackendConfig {
        provider,
        api_key: request.api_key,
        model,
        base_url: request.base_url,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        ..Default::default()
    };

    let messages: Vec<ChatMessage> = request
        .messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => Role::System,
                "assistant" => Role::Assistant,
                _ => Role::User,
            };
            ChatMessage { role, content: m.content }
        })
        .collect();

    let chat_request = ChatRequest {
        model: config.model.clone(),
        messages,
        max_tokens: Some(config.max_tokens),
        temperature: Some(config.temperature),
        stream: None,
    };

    let adapter = AiAdapter::new(config).map_err(|e| e.to_string())?;
    let response = adapter.complete(chat_request).await.map_err(|e| e.to_string())?;

    Ok(AiCompareResponse {
        content: response.content,
        prompt_tokens: response.usage.as_ref().map(|u| u.prompt_tokens),
        completion_tokens: response.usage.as_ref().map(|u| u.completion_tokens),
        total_tokens: response.usage.as_ref().map(|u| u.total_tokens),
    })
}

fn default_model_for_provider(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAI => "gpt-4o-mini",
        Provider::Anthropic => "claude-sonnet-4-20250514",
        Provider::OpenRouter => "anthropic/claude-3.5-sonnet",
        Provider::Custom => "",
    }
}

/// Reads the raw text contents of a file, truncating at `max_bytes` (defaults to 10 MiB).
/// Invalid UTF-8 is replaced with the Unicode replacement character.
#[tauri::command]
fn read_file_text(path: PathBuf, max_bytes: Option<u64>) -> Result<String, String> {
    let limit = max_bytes.unwrap_or(DEFAULT_MAX_BYTES) as usize;
    let bytes =
        std::fs::read(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let slice = &bytes[..bytes.len().min(limit)];
    Ok(String::from_utf8_lossy(slice).into_owned())
}

/// Returns true if the path exists and is a directory.
#[tauri::command]
fn path_is_dir(path: PathBuf) -> bool {
    path.is_dir()
}

/// Returns the command-line arguments this process was launched with,
/// excluding the binary name (argv[0]). The frontend uses this to
/// auto-open files or folders passed via CLI.
#[tauri::command]
fn get_launch_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            compare_files,
            compare_folders,
            compare_office,
            compare_ai,
            scan_tree,
            read_file_text,
            path_is_dir,
            get_launch_args
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("diff-tool-test");
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents).unwrap();
        path
    }

    // ── diff-progress integration tests ────────────────────────────────
    //
    // These test the DiffProgressPayload-creation logic that would be
    // emitted via `app.emit("diff-progress", …)` in the real Tauri
    // command.  We call compare_files_with_progress directly with a
    // callback that records payloads — same shared `progress_to_payload`
    // function, no mock runtime needed.

    #[test]
    fn progress_callback_produces_six_diff_progress_payloads_in_order() {
        let left = temp_file("prog_left.txt", b"line1\nline2\nline3\n");
        let right = temp_file("prog_right.txt", b"line1\nmodified\nline3\n");

        let payloads: Arc<Mutex<Vec<DiffProgressPayload>>> =
            Arc::new(Mutex::new(Vec::new()));
        let payloads_cb = payloads.clone();

        // Uses the same progress_to_payload() that the real Tauri command uses.
        let cb: Box<dyn Fn(diff_core::Progress) + Send> = Box::new(move |p| {
            payloads_cb.lock().unwrap().push(progress_to_payload(p));
        });

        let result = diff_core::compare_files_with_progress(
            DiffRequest {
                left: left.clone(),
                right: right.clone(),
                mode: DiffMode::Split,
                ignore: IgnoreMode::None,
                context: 3,
                max_bytes: Some(DEFAULT_MAX_BYTES),
                algorithm: DiffAlgorithm::Myers,
                deadline_ms: None,
                language: None,
                syntax_depth: None,
            },
            Some(cb),
        );

        assert!(
            result.is_ok(),
            "compare_files_with_progress must succeed: {:?}",
            result.err(),
        );

        let fired = payloads.lock().unwrap();
        assert_eq!(
            fired.len(),
            6,
            "must emit exactly 6 progress payloads, got {}: {:?}",
            fired.len(),
            fired.iter().map(|p| p.stage.as_str()).collect::<Vec<_>>(),
        );

        // Verify all 6 stages in order.
        let stages: Vec<&str> = fired.iter().map(|p| p.stage.as_str()).collect();
        assert_eq!(
            stages,
            vec![
                "reading",
                "reading",
                "normalizing",
                "computing_diff",
                "computing_intraline_spans",
                "building_hunks",
            ],
        );

        // Reading payloads must carry path and bytes.
        for i in 0..2 {
            assert!(
                fired[i].path.is_some(),
                "reading payload {i} must have path",
            );
            assert!(
                fired[i].bytes.is_some(),
                "reading payload {i} must have bytes",
            );
        }

        // Non-reading payloads must not carry path/bytes.
        for i in 2..6 {
            assert!(
                fired[i].path.is_none(),
                "stage '{}' must not have path",
                fired[i].stage,
            );
            assert!(
                fired[i].bytes.is_none(),
                "stage '{}' must not have bytes",
                fired[i].stage,
            );
        }

        // The diff result should be correct.
        let diff = result.unwrap();
        assert_eq!(diff.stats.deletions, 1);
        assert_eq!(diff.stats.additions, 1);
        assert_eq!(diff.stats.unchanged, 2);
    }

    #[test]
    fn compare_files_request_progress_defaults_to_false() {
        // Verify the `progress` field defaults to false via serde(default).
        // The frontend must opt in to progress events.
        let req: CompareFilesRequest =
            serde_json::from_str(r#"{"left":"/a","right":"/b"}"#).unwrap();
        assert!(!req.progress, "progress must default to false");
    }

    #[test]
    fn nonexistent_file_returns_err_with_only_reading_callbacks() {
        // When one file is missing, the Reading callback fires (it runs
        // before the actual file read), but the pipeline aborts before
        // diff computation.
        let nonexistent = PathBuf::from("/nonexistent/path/for/progress/test.xyz");
        let valid = temp_file("valid.txt", b"hello\n");

        let payloads: Arc<Mutex<Vec<DiffProgressPayload>>> =
            Arc::new(Mutex::new(Vec::new()));
        let payloads_cb = payloads.clone();

        let cb: Box<dyn Fn(diff_core::Progress) + Send> = Box::new(move |p| {
            payloads_cb.lock().unwrap().push(progress_to_payload(p));
        });

        let result = diff_core::compare_files_with_progress(
            DiffRequest {
                left: nonexistent,
                right: valid,
                mode: DiffMode::Split,
                ignore: IgnoreMode::None,
                context: 3,
                max_bytes: Some(DEFAULT_MAX_BYTES),
                algorithm: DiffAlgorithm::Myers,
                deadline_ms: None,
                language: None,
                syntax_depth: None,
            },
            Some(cb),
        );

        assert!(
            result.is_err(),
            "nonexistent file must produce an error"
        );

        let fired = payloads.lock().unwrap();
        // The Reading callback fires once for the nonexistent file
        // (with bytes=0) before the read_limited_known error aborts.
        // The valid file is never reached.
        let stages: Vec<&str> = fired.iter().map(|p| p.stage.as_str()).collect();
        assert!(
            stages.iter().all(|s| *s == "reading"),
            "only reading callbacks fire before error; got {:?}",
            stages,
        );
        assert!(
            stages.len() == 1,
            "pipeline must abort after first Reading callback on error (got {} stages: {:?})",
            stages.len(),
            stages,
        );
    }

    // ── Tauri integration tests ───────────────────────────────────────
    //
    // These tests use mock_builder() to create a real Tauri App with the
    // MockRuntime, register the compare_files command, set up event
    // listeners via app.listen(), and invoke the command directly.
    // This exercises the actual Tauri event bus, verifying that
    // app.emit("diff-progress", ...) dispatches events that listeners
    // can receive.

    /// Builds a minimal Tauri mock app with `compare_files` registered.
    fn build_mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![compare_files])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap()
    }

    #[test]
    fn diff_progress_events_are_emitted_via_tauri_event_bus() {
        let left = temp_file("tav_left.txt", b"line1\nline2\nline3\n");
        let right = temp_file("tav_right.txt", b"line1\nchanged\nline3\n");

        let app = build_mock_app();

        // Set up event listener on the Tauri event bus.
        let events: Arc<Mutex<Vec<DiffProgressPayload>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_cb = events.clone();
        let _listener = app.listen("diff-progress", move |event| {
            let payload: DiffProgressPayload =
                serde_json::from_str(event.payload()).unwrap();
            events_cb.lock().unwrap().push(payload);
        });

        let request = CompareFilesRequest {
            left,
            right,
            algorithm: None,
            deadline_ms: None,
            progress: true,
        };

        let result = compare_files(app.app_handle().clone(), request);

        assert!(
            result.is_ok(),
            "compare_files via Tauri must succeed: {:?}",
            result.err(),
        );

        let diff = result.unwrap();
        assert_eq!(diff.stats.deletions, 1);
        assert_eq!(diff.stats.additions, 1);
        assert_eq!(diff.stats.unchanged, 2);

        let fired = events.lock().unwrap();
        assert_eq!(
            fired.len(),
            6,
            "must emit exactly 6 events via Tauri bus, got {}: {:?}",
            fired.len(),
            fired.iter().map(|p| p.stage.as_str()).collect::<Vec<_>>(),
        );

        let stages: Vec<&str> = fired.iter().map(|p| p.stage.as_str()).collect();
        assert_eq!(
            stages,
            vec![
                "reading",
                "reading",
                "normalizing",
                "computing_diff",
                "computing_intraline_spans",
                "building_hunks",
            ],
            "events emitted via Tauri bus must follow correct stage order",
        );
    }

    #[test]
    fn diff_progress_disabled_emits_no_events_via_tauri_event_bus() {
        let test_file = temp_file("noev.txt", b"hello\n");

        let app = build_mock_app();

        let events: Arc<Mutex<Vec<DiffProgressPayload>>> =
            Arc::new(Mutex::new(Vec::new()));
        let events_cb = events.clone();
        let _listener = app.listen("diff-progress", move |event| {
            let payload: DiffProgressPayload =
                serde_json::from_str(event.payload()).unwrap();
            events_cb.lock().unwrap().push(payload);
        });

        let request = CompareFilesRequest {
            left: test_file.clone(),
            right: test_file,
            algorithm: None,
            deadline_ms: None,
            progress: false,
        };

        let result = compare_files(app.app_handle().clone(), request);
        assert!(
            result.is_ok(),
            "compare_files must succeed for identical files: {:?}",
            result.err(),
        );

        let fired = events.lock().unwrap();
        assert!(
            fired.is_empty(),
            "progress: false must emit zero events, got {}: {:?}",
            fired.len(),
            fired.iter().map(|p| p.stage.as_str()).collect::<Vec<_>>(),
        );
    }

    // ── existing read_file_text tests ──────────────────────────────────

    #[test]
    fn read_valid_utf8_file() {
        let path = temp_file("hello.txt", b"hello world\n");
        let result = read_file_text(path, None).unwrap();
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn read_missing_file_returns_error() {
        let result = read_file_text(PathBuf::from("/nonexistent/path.xyz"), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to read"));
    }

    #[test]
    fn truncation_at_max_bytes() {
        let path = temp_file("big.txt", b"0123456789");
        let result = read_file_text(path, Some(5)).unwrap();
        assert_eq!(result, "01234");
    }

    #[test]
    fn max_bytes_larger_than_file_reads_entire_file() {
        let path = temp_file("small.txt", b"abc");
        let result = read_file_text(path, Some(100)).unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn binary_file_is_lossy_decoded() {
        let path = temp_file("binary.bin", b"\xff\xfe\x00\x01");
        let result = read_file_text(path, None).unwrap();
        // from_utf8_lossy replaces invalid bytes with U+FFFD
        assert!(result.contains('\u{FFFD}'));
    }

    #[test]
    fn path_is_dir_returns_true_for_directory() {
        let dir = std::env::temp_dir().join("diff-tool-test");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(path_is_dir(dir));
    }

    #[test]
    fn path_is_dir_returns_false_for_file() {
        let path = temp_file("regular.txt", b"content");
        assert!(!path_is_dir(path));
    }

    #[test]
    fn path_is_dir_returns_false_for_nonexistent() {
        assert!(!path_is_dir(PathBuf::from("/nonexistent/path.xyz")));
    }

    #[test]
    fn get_launch_args_returns_vec() {
        // In test context args() includes the test binary name followed by
        // test harness flags. We only check that it returns a Vec (may be empty).
        let args = get_launch_args();
        // The result is a Vec<String> — no further assertions needed.
        let _: Vec<String> = args;
    }

    #[test]
    fn compare_folders_command_success() {
        let left = temp_file("tauri_left/file.txt", b"hello world");
        let right = temp_file("tauri_right/file.txt", b"hello world");
        let left_dir = left.parent().unwrap().to_path_buf();
        let right_dir = right.parent().unwrap().to_path_buf();

        let result = compare_folders(left_dir, right_dir).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].status, git_adapter::EntryStatus::Unchanged);
    }

    #[test]
    fn compare_folders_command_invalid_path_fails() {
        let result = compare_folders(
            PathBuf::from("/nonexistent/left_folder_xyz"),
            PathBuf::from("/nonexistent/right_folder_xyz"),
        );
        assert!(result.is_err());
    }
}
