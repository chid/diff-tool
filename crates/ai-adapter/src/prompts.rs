//! Prompt templates for AI features.
//!
//! Each template produces a `ChatRequest` from feature-specific input data.
//! Templates are versioned — bumping a version invalidates the cache for
//! that prompt type (see `cache.rs`).
//!
//! These templates will be wired into the Tauri backend in Phase 2.
#![allow(dead_code)]

use crate::types::{
    AiChangeSummary, AiCommitMessage, AiMergeResolution, ChatMessage, ChatRequest,
    FileCluster, FileRiskAssessment, ReviewQuestion,
};

// ── Prompt template versions (bump to invalidate cache) ────────────────────

pub const SUMMARY_PROMPT_VERSION: u32 = 1;
pub const MERGE_PROMPT_VERSION: u32 = 1;
pub const RISK_PROMPT_VERSION: u32 = 1;
pub const CLUSTER_PROMPT_VERSION: u32 = 1;
pub const REVIEW_QUESTIONS_PROMPT_VERSION: u32 = 1;
pub const COMMIT_MSG_PROMPT_VERSION: u32 = 1;
pub const EXPLAIN_HUNK_PROMPT_VERSION: u32 = 1;

// ── Change summary ────────────────────────────────────────────────────────

/// Build a chat request for generating a change summary from a unified diff.
pub fn build_summary_request(
    model: &str,
    diff_text: &str,
    language: &str,
) -> ChatRequest {
    let system = format!(
        "You are a code review assistant. Given a unified diff, produce a concise \
         summary of what changed. The diff language is {language}."
    );

    let user = format!(
        "Summarize the following diff. Respond with a JSON object containing:\n\
         - \"summary\": one sentence describing the change.\n\
         - \"intents\": an array of 2-5 bullet points explaining the intent of each change.\n\
         - \"risks\": an array of potential risks or concerns (empty if none).\n\n\
         ```diff\n{diff_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(512),
        temperature: Some(0.3),
        stream: None,
    }
}

// ── Merge conflict resolution ─────────────────────────────────────────────

/// Build a chat request for AI-assisted merge conflict resolution.
pub fn build_merge_request(
    model: &str,
    base: &str,
    ours: &str,
    theirs: &str,
    language: &str,
) -> ChatRequest {
    let system = format!(
        "You are a merge conflict resolution assistant. Given the base, ours, and \
         theirs versions of a conflicted file, produce a fully resolved version. \
         The file language is {language}. For each conflict hunk, explain your \
         reasoning."
    );

    let user = format!(
        "Resolve the merge conflicts in the following file. Respond with a JSON object \
         containing:\n\
         - \"resolved_file\": the fully resolved file content as a single string.\n\
         - \"hunks\": an array of objects, each with:\n\
           - \"resolved_text\": the resolved text for this hunk.\n\
           - \"explanation\": why this resolution was chosen.\n\
           - \"ambiguous\": true if the correct resolution is genuinely ambiguous.\n\n\
         BASE:\n```{language}\n{base}\n```\n\n\
         OURS:\n```{language}\n{ours}\n```\n\n\
         THEIRS:\n```{language}\n{theirs}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(2048),
        temperature: Some(0.2),
        stream: None,
    }
}

// ── Risk assessment ───────────────────────────────────────────────────────

/// Build a chat request for file-level risk assessment.
pub fn build_risk_request(
    model: &str,
    file_path: &str,
    diff_text: &str,
    language: &str,
) -> ChatRequest {
    let system = "You are a code review assistant. Assess the risk level of a code \
                  change and identify potentially affected areas.";

    let user = format!(
        "Assess the risk of the following change to `{file_path}`. Respond with a JSON \
         object containing:\n\
         - \"risk_level\": one of \"low\", \"medium\", or \"high\".\n\
         - \"justification\": a short justification for the risk level.\n\
         - \"affected_areas\": an array of files, modules, or concepts that may be \
           affected by this change (empty if none).\n\n\
         ```{language}\n{diff_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(512),
        temperature: Some(0.2),
        stream: None,
    }
}

// ── Intent clustering ─────────────────────────────────────────────────────

/// Build a chat request for clustering changed files by intent/theme.
pub fn build_cluster_request(
    model: &str,
    file_diffs: &[(String, Option<String>)],
) -> ChatRequest {
    let file_list: String = file_diffs
        .iter()
        .map(|(path, summary)| {
            if let Some(s) = summary {
                format!("- {path}: {s}")
            } else {
                format!("- {path}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let system = "You are a code review assistant. Group changed files by the \
                  intent or theme of the change, not by directory path.";

    let user = format!(
        "Group the following changed files into clusters by the intent of the change. \
         Respond with a JSON array of clusters, each with:\n\
         - \"theme\": a short name for the cluster (e.g. \"Auth refactor\").\n\
         - \"description\": a one-sentence description.\n\
         - \"file_paths\": an array of file paths in this cluster.\n\n\
         Files:\n{file_list}"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(1024),
        temperature: Some(0.3),
        stream: None,
    }
}

// ── Review questions ──────────────────────────────────────────────────────

/// Build a chat request for generating review questions from a diff.
pub fn build_review_questions_request(
    model: &str,
    diff_text: &str,
    language: &str,
) -> ChatRequest {
    let system = "You are a code review assistant. Generate questions a reviewer \
                  should ask the author about their change.";

    let user = format!(
        "Based on the following diff, generate 3-5 questions a reviewer should ask \
         the author. Focus on edge cases, missing tests, security concerns, and \
         architectural mismatches. Respond with a JSON array of objects, each with:\n\
         - \"question\": the question text.\n\
         - \"context\": optional file or line reference.\n\n\
         ```{language}\n{diff_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(512),
        temperature: Some(0.4),
        stream: None,
    }
}

// ── Commit message ────────────────────────────────────────────────────────

/// Build a chat request for generating a conventional commit message.
pub fn build_commit_msg_request(model: &str, diff_text: &str) -> ChatRequest {
    let system = "You are a commit message generator. Produce conventional commit \
                  messages from diffs.";

    let user = format!(
        "Generate a conventional commit message for the following diff. Respond with \
         a JSON object containing:\n\
         - \"subject\": the conventional commit subject line (e.g. \"feat(auth): ...\").\n\
         - \"body\": an optional body with bullet points explaining the changes.\n\n\
         ```diff\n{diff_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(512),
        temperature: Some(0.3),
        stream: None,
    }
}

// ── Explain hunk ──────────────────────────────────────────────────────────

/// Build a chat request for explaining a specific diff hunk or line.
pub fn build_explain_hunk_request(
    model: &str,
    hunk_text: &str,
    language: &str,
) -> ChatRequest {
    let system = format!(
        "You are a code explanation assistant. Given a diff hunk, explain what the \
         change does and why it might have been made. The language is {language}."
    );

    let user = format!(
        "Explain the following diff hunk in one paragraph. Focus on what changed and \
         why, not a line-by-line description.\n\n\
         ```{language}\n{hunk_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(256),
        temperature: Some(0.3),
        stream: None,
    }
}

// ── Response parsers ──────────────────────────────────────────────────────

/// Parse an LLM JSON response into an `AiChangeSummary`.
pub fn parse_summary_response(json: &str) -> Result<AiChangeSummary, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse summary: {e}"))
}

/// Parse an LLM JSON response into an `AiMergeResolution`.
pub fn parse_merge_response(json: &str) -> Result<AiMergeResolution, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse merge: {e}"))
}

/// Parse an LLM JSON response into a `FileRiskAssessment`.
pub fn parse_risk_response(json: &str) -> Result<FileRiskAssessment, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse risk: {e}"))
}

/// Parse an LLM JSON response into a `Vec<FileCluster>`.
pub fn parse_cluster_response(json: &str) -> Result<Vec<FileCluster>, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse clusters: {e}"))
}

/// Parse an LLM JSON response into a `Vec<ReviewQuestion>`.
pub fn parse_review_questions_response(json: &str) -> Result<Vec<ReviewQuestion>, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse questions: {e}"))
}

/// Parse an LLM JSON response into an `AiCommitMessage`.
pub fn parse_commit_msg_response(json: &str) -> Result<AiCommitMessage, String> {
    serde_json::from_str(json).map_err(|e| format!("failed to parse commit: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RiskLevel;

    #[test]
    fn build_summary_request_includes_diff_text() {
        let req = build_summary_request("gpt-4o", "--- a/foo.rs\n+++ b/foo.rs\n", "rust");
        assert_eq!(req.model, "gpt-4o");
        assert_eq!(req.messages.len(), 2);
        assert!(req.messages[1].content.contains("--- a/foo.rs"));
    }

    #[test]
    fn build_merge_request_includes_all_three_sides() {
        let req = build_merge_request(
            "claude-3",
            "base content",
            "ours content",
            "theirs content",
            "go",
        );
        assert_eq!(req.messages.len(), 2);
        let user = &req.messages[1].content;
        assert!(user.contains("base content"));
        assert!(user.contains("ours content"));
        assert!(user.contains("theirs content"));
        assert!(user.contains("go"));
    }

    #[test]
    fn parse_summary_response_valid_json() {
        let json = r#"{"summary":"changed X","intents":["did Y"],"risks":[]}"#;
        let result = parse_summary_response(json).unwrap();
        assert_eq!(result.summary, "changed X");
        assert_eq!(result.intents, vec!["did Y"]);
        assert!(result.risks.is_empty());
    }

    #[test]
    fn parse_summary_response_invalid_json_returns_error() {
        let result = parse_summary_response("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_risk_response_all_levels() {
        for level in &["low", "medium", "high"] {
            let json = format!(
                r#"{{"risk_level":"{level}","justification":"ok","affected_areas":[]}}"#
            );
            let result = parse_risk_response(&json).unwrap();
            match *level {
                "low" => assert_eq!(result.risk_level, RiskLevel::Low),
                "medium" => assert_eq!(result.risk_level, RiskLevel::Medium),
                "high" => assert_eq!(result.risk_level, RiskLevel::High),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn build_commit_msg_request_outputs_conventional_format() {
        let req = build_commit_msg_request("gpt-4o", "--- a/src/main.rs\n+++ b/src/main.rs\n");
        assert!(req.messages[1].content.contains("conventional commit"));
        assert!(req.messages[1].content.contains("subject"));
    }
}
