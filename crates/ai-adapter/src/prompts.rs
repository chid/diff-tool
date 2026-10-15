//! Prompt templates for AI features.
//!
//! Each template produces a `ChatRequest` from feature-specific input data.
//! Templates are versioned — bumping a version invalidates the cache for
//! that prompt type (see `cache.rs`).
//!
//! These templates will be wired into the Tauri backend in Phase 2.
#![allow(dead_code)]

use crate::types::{
    AiChangeSummary, AiCommitMessage, AiFullReviewReport, AiHunkExplanation,
    AiMergeResolution, ChatMessage, ChatRequest, FileCluster, FileRiskAssessment,
    PromptCategory, PromptPackageMetadata, ReviewQuestion,
};

// ── Prompt template versions (bump to invalidate cache) ────────────────────

pub const SUMMARY_PROMPT_VERSION: u32 = 1;
pub const FULL_REVIEW_PROMPT_VERSION: u32 = 1;
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

// ── Full code review ──────────────────────────────────────────────────────

/// Build a chat request for a comprehensive code review (summary + risk + reviewer questions + commit message).
pub fn build_full_review_request(
    model: &str,
    diff_text: &str,
    language: &str,
) -> ChatRequest {
    let system = format!(
        "You are an expert senior code reviewer and software architect. Given a unified diff, \
         conduct a thorough, structured review. Focus on intent, correctness, security, architectural \
         coherence, and test coverage. The diff language is {language}."
    );

    let user = format!(
        "Analyze the following diff and provide a complete code review report. Respond with a valid JSON object containing:\n\
         - \"summary\": object with:\n\
           - \"summary\": one concise sentence describing the primary change.\n\
           - \"intents\": array of 2-5 bullet points explaining the core architectural intent.\n\
           - \"risks\": array of potential risk factors or regression concerns.\n\
         - \"risk\": object with:\n\
           - \"risk_level\": \"low\", \"medium\", or \"high\".\n\
           - \"justification\": brief explanation for the rating.\n\
           - \"affected_areas\": array of components, modules, or services affected.\n\
         - \"questions\": array of 3-5 critical questions a reviewer should ask the author.\n\
           Each question is an object with \"question\" (string) and optional \"context\" (string).\n\
         - \"suggested_commit\": object with \"subject\" (conventional commit format) and \"body\" (optional).\n\n\
         ```diff\n{diff_text}\n```"
    );

    ChatRequest {
        model: model.into(),
        messages: vec![ChatMessage::system(system), ChatMessage::user(user)],
        max_tokens: Some(1536),
        temperature: Some(0.25),
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

fn strip_json_fences(input: &str) -> &str {
    let mut s = input.trim();
    if s.starts_with("```") {
        if let Some(newline) = s.find('\n') {
            s = &s[newline + 1..];
        }
        if let Some(end) = s.rfind("```") {
            s = &s[..end];
        }
    }
    s.trim()
}

/// Parse an LLM JSON response into an `AiChangeSummary`.
pub fn parse_summary_response(json: &str) -> Result<AiChangeSummary, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse summary: {e}"))
}

/// Parse an LLM JSON response into an `AiMergeResolution`.
pub fn parse_merge_response(json: &str) -> Result<AiMergeResolution, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse merge: {e}"))
}

/// Parse an LLM JSON response into a `FileRiskAssessment`.
pub fn parse_risk_response(json: &str) -> Result<FileRiskAssessment, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse risk: {e}"))
}

/// Parse an LLM JSON response into a `Vec<FileCluster>`.
pub fn parse_cluster_response(json: &str) -> Result<Vec<FileCluster>, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse clusters: {e}"))
}

/// Parse an LLM JSON response into a `Vec<ReviewQuestion>`.
pub fn parse_review_questions_response(json: &str) -> Result<Vec<ReviewQuestion>, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse questions: {e}"))
}

/// Parse an LLM JSON response into an `AiCommitMessage`.
pub fn parse_commit_msg_response(json: &str) -> Result<AiCommitMessage, String> {
    serde_json::from_str(strip_json_fences(json)).map_err(|e| format!("failed to parse commit: {e}"))
}

/// Parse an LLM JSON response into an `AiFullReviewReport`.
pub fn parse_full_review_response(json: &str) -> Result<AiFullReviewReport, String> {
    let clean = strip_json_fences(json);
    if let Ok(rep) = serde_json::from_str::<AiFullReviewReport>(clean) {
        return Ok(rep);
    }

    // Handle models that wrap fields inside a nested "summary", "review", or "report" object
    let val: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(e) => return Err(format!("failed to parse full review JSON: {e}")),
    };

    let root = if let Some(r) = val.get("review") {
        r
    } else if let Some(r) = val.get("report") {
        r
    } else {
        &val
    };

    if let Ok(rep) = serde_json::from_value::<AiFullReviewReport>(root.clone()) {
        return Ok(rep);
    }

    if let Some(summary_obj) = root.get("summary") {
        if summary_obj.get("risk").is_some() || summary_obj.get("questions").is_some() {
            let summary_text = summary_obj
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let intents: Vec<String> = summary_obj
                .get("intents")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let risks: Vec<String> = summary_obj
                .get("risks")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();

            let risk: FileRiskAssessment = summary_obj
                .get("risk")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .or_else(|| root.get("risk").and_then(|v| serde_json::from_value(v.clone()).ok()))
                .ok_or_else(|| "missing field `risk` in response".to_string())?;

            let questions: Vec<ReviewQuestion> = summary_obj
                .get("questions")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .or_else(|| root.get("questions").and_then(|v| serde_json::from_value(v.clone()).ok()))
                .unwrap_or_default();

            let suggested_commit: Option<AiCommitMessage> = summary_obj
                .get("suggested_commit")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .or_else(|| root.get("suggested_commit").and_then(|v| serde_json::from_value(v.clone()).ok()));

            return Ok(AiFullReviewReport {
                summary: AiChangeSummary {
                    summary: summary_text,
                    intents,
                    risks,
                },
                risk,
                questions,
                suggested_commit,
            });
        }
    }

    serde_json::from_str(clean).map_err(|e| format!("failed to parse full review: {e}"))
}

/// Parse an LLM response into an `AiHunkExplanation`.
pub fn parse_explain_hunk_response(text_or_json: &str) -> Result<AiHunkExplanation, String> {
    let clean = strip_json_fences(text_or_json);
    if let Ok(parsed) = serde_json::from_str::<AiHunkExplanation>(clean) {
        return Ok(parsed);
    }
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        return Err("empty explanation response".into());
    }
    Ok(AiHunkExplanation {
        explanation: trimmed.to_string(),
        key_points: Vec::new(),
    })
}

// ── Prompt Package Registry ───────────────────────────────────────────────

/// Return metadata for all standard, ready-to-run prompt packages.
pub fn list_prompt_packages() -> Vec<PromptPackageMetadata> {
    vec![
        PromptPackageMetadata {
            id: "full-review".into(),
            name: "Comprehensive Code Review".into(),
            description: "All-in-one report: executive summary, risk level, reviewer questions, and conventional commit suggestion.".into(),
            category: PromptCategory::Review,
            version: FULL_REVIEW_PROMPT_VERSION,
            default_temperature: 0.25,
            default_max_tokens: 1536,
            tags: vec!["review".into(), "summary".into(), "risk".into(), "questions".into()],
        },
        PromptPackageMetadata {
            id: "change-summary".into(),
            name: "Executive Change Summary".into(),
            description: "High-level summary of intent, bulleted changes, and notable risk signals.".into(),
            category: PromptCategory::Review,
            version: SUMMARY_PROMPT_VERSION,
            default_temperature: 0.3,
            default_max_tokens: 512,
            tags: vec!["summary".into(), "intent".into(), "tldr".into()],
        },
        PromptPackageMetadata {
            id: "risk-assessment".into(),
            name: "Risk & Blast Radius Audit".into(),
            description: "Evaluates risk level (low/medium/high), justification, and affected modules or dependencies.".into(),
            category: PromptCategory::Audit,
            version: RISK_PROMPT_VERSION,
            default_temperature: 0.2,
            default_max_tokens: 512,
            tags: vec!["risk".into(), "security".into(), "impact".into(), "audit".into()],
        },
        PromptPackageMetadata {
            id: "review-questions".into(),
            name: "Targeted Reviewer Questions".into(),
            description: "Generates 3-5 critical questions focusing on edge cases, missing test fixtures, and security concerns.".into(),
            category: PromptCategory::Review,
            version: REVIEW_QUESTIONS_PROMPT_VERSION,
            default_temperature: 0.4,
            default_max_tokens: 512,
            tags: vec!["questions".into(), "review".into(), "checklist".into()],
        },
        PromptPackageMetadata {
            id: "commit-message".into(),
            name: "Conventional Commit Generator".into(),
            description: "Generates a structured conventional commit message (feat, fix, refactor) from staged or file diffs.".into(),
            category: PromptCategory::Authoring,
            version: COMMIT_MSG_PROMPT_VERSION,
            default_temperature: 0.3,
            default_max_tokens: 512,
            tags: vec!["commit".into(), "git".into(), "conventional".into()],
        },
        PromptPackageMetadata {
            id: "explain-hunk".into(),
            name: "Hunk / Line Change Explainer".into(),
            description: "Explains a specific diff hunk or selected code change in plain English.".into(),
            category: PromptCategory::Explain,
            version: EXPLAIN_HUNK_PROMPT_VERSION,
            default_temperature: 0.3,
            default_max_tokens: 256,
            tags: vec!["explain".into(), "hunk".into(), "education".into()],
        },
        PromptPackageMetadata {
            id: "intent-cluster".into(),
            name: "Multi-File Intent Clustering".into(),
            description: "Groups changed files by architectural intent rather than directory layout.".into(),
            category: PromptCategory::Audit,
            version: CLUSTER_PROMPT_VERSION,
            default_temperature: 0.3,
            default_max_tokens: 1024,
            tags: vec!["cluster".into(), "theme".into(), "multi-file".into()],
        },
        PromptPackageMetadata {
            id: "merge-conflict".into(),
            name: "Merge Conflict Resolution".into(),
            description: "3-way conflict analysis suggesting resolved code with reasoning and ambiguity flags.".into(),
            category: PromptCategory::Merge,
            version: MERGE_PROMPT_VERSION,
            default_temperature: 0.2,
            default_max_tokens: 2048,
            tags: vec!["merge".into(), "conflict".into(), "git".into()],
        },
    ]
}

/// Build a `ChatRequest` for a named prompt package.
pub fn build_package_request(
    package_id: &str,
    model: &str,
    diff_text: &str,
    language: Option<&str>,
    file_path: Option<&str>,
) -> Result<ChatRequest, String> {
    let lang = language.unwrap_or("text");
    match package_id {
        "full-review" => Ok(build_full_review_request(model, diff_text, lang)),
        "change-summary" | "summary" => Ok(build_summary_request(model, diff_text, lang)),
        "risk-assessment" | "risk" => {
            let path = file_path.unwrap_or("file");
            Ok(build_risk_request(model, path, diff_text, lang))
        }
        "review-questions" | "questions" => Ok(build_review_questions_request(model, diff_text, lang)),
        "commit-message" | "commit" => Ok(build_commit_msg_request(model, diff_text)),
        "explain-hunk" | "explain" => Ok(build_explain_hunk_request(model, diff_text, lang)),
        _ => Err(format!(
            "unknown prompt package '{package_id}'. Available: {}",
            list_prompt_packages()
                .iter()
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
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

    #[test]
    fn build_full_review_request_has_all_sections() {
        let req = build_full_review_request("gpt-4o", "--- a/lib.rs\n+++ b/lib.rs\n", "rust");
        assert_eq!(req.messages.len(), 2);
        let user = &req.messages[1].content;
        assert!(user.contains("summary"));
        assert!(user.contains("risk"));
        assert!(user.contains("questions"));
        assert!(user.contains("suggested_commit"));
    }

    #[test]
    fn parse_full_review_response_valid() {
        let json = r#"{
            "summary": { "summary": "Refactored auth", "intents": ["added jwt"], "risks": [] },
            "risk": { "risk_level": "medium", "justification": "crypto logic", "affected_areas": ["api"] },
            "questions": [{ "question": "Are tokens rotated?", "context": "auth.rs:42" }],
            "suggested_commit": { "subject": "feat(auth): add jwt rotation", "body": "details" }
        }"#;
        let res = parse_full_review_response(json).unwrap();
        assert_eq!(res.summary.summary, "Refactored auth");
        assert_eq!(res.risk.risk_level, RiskLevel::Medium);
        assert_eq!(res.questions.len(), 1);
        assert_eq!(res.suggested_commit.unwrap().subject, "feat(auth): add jwt rotation");
    }

    #[test]
    fn parse_explain_hunk_response_plain_text_and_json() {
        let plain = "This change inlines the helper function for performance.";
        let res1 = parse_explain_hunk_response(plain).unwrap();
        assert_eq!(res1.explanation, plain);

        let json = r#"{"explanation": "Updated timeout", "key_points": ["30s to 60s"]}"#;
        let res2 = parse_explain_hunk_response(json).unwrap();
        assert_eq!(res2.explanation, "Updated timeout");
        assert_eq!(res2.key_points, vec!["30s to 60s"]);
    }

    #[test]
    fn list_prompt_packages_returns_all_expected() {
        let packages = list_prompt_packages();
        assert!(packages.len() >= 8);
        let ids: Vec<&str> = packages.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"full-review"));
        assert!(ids.contains(&"change-summary"));
        assert!(ids.contains(&"risk-assessment"));
        assert!(ids.contains(&"review-questions"));
        assert!(ids.contains(&"commit-message"));
        assert!(ids.contains(&"explain-hunk"));
        assert!(ids.contains(&"intent-cluster"));
        assert!(ids.contains(&"merge-conflict"));
    }

    #[test]
    fn build_package_request_routes_correctly() {
        let req1 = build_package_request("full-review", "gpt-4o", "diff", Some("rust"), None).unwrap();
        assert_eq!(req1.model, "gpt-4o");

        let req2 = build_package_request("risk", "claude-3", "diff", None, Some("src/main.rs")).unwrap();
        assert!(req2.messages[1].content.contains("src/main.rs"));

        let err = build_package_request("nonexistent", "gpt-4o", "diff", None, None);
        assert!(err.is_err());
    }
}

