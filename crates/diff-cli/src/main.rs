mod config;

use ai_adapter::{AiAdapter, BackendConfig, Provider, RiskLevel};
use clap::{Parser, Subcommand, ValueEnum};
use diff_core::{DiffAlgorithm, DiffError, DiffMode, DiffRequest, IgnoreMode};
use git_adapter::EntryStatus;
use rayon::prelude::*;

use serde::Serialize;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum AlgorithmArg {
    Myers,
    Patience,
    Histogram,
    Syntax,
}

impl From<AlgorithmArg> for DiffAlgorithm {
    fn from(arg: AlgorithmArg) -> Self {
        match arg {
            AlgorithmArg::Myers => DiffAlgorithm::Myers,
            AlgorithmArg::Patience => DiffAlgorithm::Patience,
            AlgorithmArg::Histogram => DiffAlgorithm::Histogram,
            AlgorithmArg::Syntax => DiffAlgorithm::Syntax,
        }
    }
}

/// Exit code constants (documented in `dt --help`).
const EXIT_SUCCESS: i32 = 0;
const EXIT_DIFFERENCES: i32 = 1;
const EXIT_ERROR: i32 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "dt",
    about = "Diff Tool command-line integration",
    after_long_help = "\
EXIT CODES:\n  \
  0  Success (files identical, or operation completed)\n  \
  1  Differences found\n  \
  2  Error (unreadable file, binary file, invalid argument, etc.)\n"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Diff {
        left: PathBuf,
        right: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, conflicts_with = "ignore_whitespace_and_case")]
        ignore_whitespace: bool,
        #[arg(long, conflicts_with = "ignore_whitespace")]
        ignore_whitespace_and_case: bool,
        #[arg(long, default_value = "3")]
        context: usize,
        #[arg(long, value_enum)]
        algorithm: Option<AlgorithmArg>,
        /// Wall-clock deadline in milliseconds. When set, diff computation
        /// is interrupted if it exceeds this budget.  Returns a best-effort
        /// result with empty hunks and a warning.
        #[arg(long)]
        deadline: Option<u64>,
        /// Print progress messages to stderr during diff computation.
        #[arg(long)]
        progress: bool,
        /// Override language for syntax-aware AST diffing.
        /// Supported: rust, typescript, python.
        #[arg(long)]
        language: Option<String>,
        /// AST node depth for syntax-aware diffing.
        /// Levels: statement (default), function, token.
        #[arg(long)]
        syntax_depth: Option<String>,
    },
    /// Render a terminal UI scaffold for a two-file diff.
    ///
    /// This first slice is deterministic and non-interactive so it can run in
    /// CI and snapshots. It establishes the split-pane information architecture
    /// before the interactive terminal backend lands.
    Tui {
        left: PathBuf,
        right: PathBuf,
        #[arg(long, conflicts_with = "ignore_whitespace_and_case")]
        ignore_whitespace: bool,
        #[arg(long, conflicts_with = "ignore_whitespace")]
        ignore_whitespace_and_case: bool,
        #[arg(long, default_value = "3")]
        context: usize,
        #[arg(long, value_enum)]
        algorithm: Option<AlgorithmArg>,
        /// Wall-clock deadline in milliseconds.
        #[arg(long)]
        deadline: Option<u64>,
        /// Override language for syntax-aware AST diffing.
        /// Supported: rust, typescript, python.
        #[arg(long)]
        language: Option<String>,
        /// AST node depth for syntax-aware diffing.
        /// Levels: statement (default), function, token.
        #[arg(long)]
        syntax_depth: Option<String>,
        /// Preview width in terminal columns.
        #[arg(long, default_value = "120")]
        width: usize,
        /// Maximum hunks to render in the scaffold preview.
        #[arg(long, default_value = "8")]
        max_hunks: usize,
        /// Maximum lines per hunk to render in the scaffold preview.
        #[arg(long, default_value = "80")]
        max_lines_per_hunk: usize,
    },
    /// Scan a directory tree, or compare two directories.
    ///
    /// With only ROOT: lists all files in the tree (like `ls -R`).
    /// With --right: diffs matching file pairs across left and right roots in
    /// parallel, reporting additions, deletions, and modifications.
    Dir {
        root: PathBuf,
        /// Optional right-hand directory for two-folder comparison: `dt dir <left> <right>`.
        #[arg(value_name = "RIGHT")]
        right_pos: Option<PathBuf>,
        /// Right-hand directory for two-folder comparison (`--right <path>`).
        #[arg(long)]
        right: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum)]
        algorithm: Option<AlgorithmArg>,
        /// Wall-clock deadline per file in milliseconds.
        #[arg(long)]
        deadline: Option<u64>,
        /// Print progress messages to stderr during diff computation.
        /// Each message is prefixed with the relative file path.
        #[arg(long)]
        progress: bool,
        /// Override language for syntax-aware AST diffing.
        /// Supported: rust, typescript, python.
        #[arg(long)]
        language: Option<String>,
        /// AST node depth for syntax-aware diffing.
        /// Levels: statement (default), function, token.
        #[arg(long)]
        syntax_depth: Option<String>,
    },
    Merge {
        #[arg(long)]
        base: PathBuf,
        #[arg(long)]
        ours: PathBuf,
        #[arg(long)]
        theirs: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    /// Compare Office documents (.docx, .pptx, .xlsx).
    Office {
        #[command(subcommand)]
        command: OfficeCommand,
    },
    /// AI-powered diff analysis (summaries, commit messages, risk assessment).
    Ai {
        /// LLM provider: openai, anthropic, or custom.
        #[arg(long, default_value = "openai")]
        provider: String,
        /// API key (or set OPENAI_API_KEY / ANTHROPIC_API_KEY env var).
        #[arg(long)]
        api_key: Option<String>,
        /// Model name (provider-specific default if omitted).
        #[arg(long)]
        model: Option<String>,
        /// Base URL override (required for custom provider).
        #[arg(long)]
        base_url: Option<String>,
        /// Read diff from file instead of stdin.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Output as JSON (default for AI commands).
        #[arg(long)]
        json: bool,
        #[command(subcommand)]
        command: AiCommand,
    },
    /// Manage persistent CLI configuration (~/.config/diff-tool/config.toml).
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum OfficeCommand {
    /// Compare two Office documents.
    Diff {
        left: PathBuf,
        right: PathBuf,
        /// Output as JSON instead of human-readable summary.
        #[arg(long)]
        json: bool,
        /// For Excel: only compare these sheets (comma-separated).
        #[arg(long, value_delimiter = ',')]
        sheets: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum AiCommand {
    /// Generate a change summary from a diff.
    GenerateSummary {
        /// Language of the changed code (affects prompt phrasing).
        #[arg(long, default_value = "text")]
        language: String,
    },
    /// Comprehensive code review report (summary, risk assessment, reviewer questions, and commit suggestion).
    FullReview {
        /// Language of the changed code.
        #[arg(long, default_value = "text")]
        language: String,
    },
    /// Generate a conventional commit message from a diff.
    GenerateCommitMsg,
    /// Assess the risk level of a code change.
    AssessRisk {
        /// The file path the diff applies to.
        #[arg(long, default_value = "file")]
        file_path: String,
        /// Language of the changed code.
        #[arg(long, default_value = "text")]
        language: String,
    },
    /// Generate targeted reviewer questions for the author.
    ReviewQuestions {
        /// Language of the changed code.
        #[arg(long, default_value = "text")]
        language: String,
    },
    /// Explain a diff hunk or code selection in plain English.
    ExplainHunk {
        /// Language of the changed code.
        #[arg(long, default_value = "text")]
        language: String,
    },
    /// List all available prompt packages with versions and metadata.
    ListPackages,
    /// Execute a named prompt package (e.g. "full-review", "change-summary", "risk-assessment").
    Run {
        /// Name or ID of the prompt package.
        package: String,
        /// Optional language override.
        #[arg(long)]
        language: Option<String>,
        /// Optional file path context.
        #[arg(long)]
        file_path: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Print the current configuration.
    Get,
    /// Set a configuration value.
    Set {
        /// Key to set: "algorithm" or "deadline".
        key: String,
        /// Value to set.
        value: String,
    },
    /// Remove the config file (reset to defaults).
    Reset,
}

#[derive(Debug, Subcommand)]
enum GitCommand {
    Install,
}

fn main() {
    let args = Args::parse();
    let code = run(args);
    process::exit(code);
}

fn run(args: Args) -> i32 {
    match args.command {
        Command::Diff {
            left,
            right,
            json,
            ignore_whitespace,
            ignore_whitespace_and_case,
            context,
            algorithm,
            deadline,
            progress,
            language,
            syntax_depth,
        } => {
            // Merge config defaults: CLI flags > config > hardcoded defaults.
            let cfg = config::load();
            let algorithm = algorithm
                .or_else(|| {
                    cfg.defaults
                        .algorithm
                        .as_deref()
                        .and_then(config::parse_algorithm)
                })
                .unwrap_or(AlgorithmArg::Myers);
            let deadline = deadline.or(cfg.defaults.deadline);

            if algorithm == AlgorithmArg::Syntax {
                eprintln!("warning: experimental --algorithm syntax (see ROUGH_EDGES.md)");
            }

            let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> = if progress {
                Some(Box::new(|p| match p {
                    diff_core::Progress::Reading { path, bytes } => {
                        let kb = bytes / 1024;
                        eprintln!("  reading {} ({} KiB)", path.display(), kb);
                    }
                    diff_core::Progress::Normalizing => {
                        eprintln!("  normalizing...");
                    }
                    diff_core::Progress::ComputingDiff => {
                        eprintln!("  computing diff...");
                    }
                    diff_core::Progress::ComputingIntralineSpans => {
                        eprintln!("  computing intraline spans...");
                    }
                    diff_core::Progress::BuildingHunks => {
                        eprintln!("  building hunks...");
                    }
                }))
            } else {
                None
            };

            let result = diff_core::compare_files_with_progress(
                DiffRequest {
                    left,
                    right,
                    mode: DiffMode::Split,
                    ignore: if ignore_whitespace_and_case {
                        IgnoreMode::WhitespaceAndCase
                    } else if ignore_whitespace {
                        IgnoreMode::Whitespace
                    } else {
                        IgnoreMode::None
                    },
                    context,
                    max_bytes: Some(10 * 1024 * 1024),
                    algorithm: algorithm.into(),
                    deadline_ms: deadline,
                    language,
                    syntax_depth,
                },
                progress_cb,
            );
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    eprint_error(&e);
                    return EXIT_ERROR;
                }
            };
            // Print warnings to stderr so they don't corrupt JSON/stdout.
            for w in &result.warnings {
                eprintln!("warning: {w}");
            }
            if json {
                match serde_json::to_string_pretty(&result) {
                    Ok(json) => println!("{json}"),
                    Err(e) => {
                        eprintln!("error: failed to serialise JSON: {e}");
                        return EXIT_ERROR;
                    }
                }
            } else {
                println!(
                    "{} additions, {} deletions, {} unchanged",
                    result.stats.additions, result.stats.deletions, result.stats.unchanged
                );
            }
            if result.stats.additions == 0 && result.stats.deletions == 0 {
                EXIT_SUCCESS
            } else {
                EXIT_DIFFERENCES
            }
        }
        Command::Dir {
            root,
            right_pos,
            right,
            json,
            algorithm,
            deadline,
            progress,
            language,
            syntax_depth,
        } => {
            if !root.exists() {
                eprintln!("error: {} does not exist", root.display());
                return EXIT_ERROR;
            }
            if !root.is_dir() {
                eprintln!("error: {} is not a directory", root.display());
                return EXIT_ERROR;
            }

            // Positional two-folder comparison: `dt dir <left> <right>`
            if let Some(right_root) = right_pos {
                return run_compare_folders(root, right_root, json);
            }

            // Merge config defaults for algorithm and deadline.
            let cfg = config::load();
            let algorithm = algorithm
                .or_else(|| {
                    cfg.defaults
                        .algorithm
                        .as_deref()
                        .and_then(config::parse_algorithm)
                })
                .unwrap_or(AlgorithmArg::Myers);
            let deadline = deadline.or(cfg.defaults.deadline);

            if algorithm == AlgorithmArg::Syntax {
                eprintln!("warning: experimental --algorithm syntax (see ROUGH_EDGES.md)");
            }

            if let Some(right_root) = right {
                return run_dir_diff(
                    root,
                    right_root,
                    json,
                    algorithm,
                    deadline,
                    progress,
                    language,
                    syntax_depth,
                );
            }

            // Single-root listing (backward compatible).
            let result = match git_adapter::scan_tree(&root) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {e}");
                    return EXIT_ERROR;
                }
            };
            if json {
                match serde_json::to_string_pretty(&result) {
                    Ok(json) => println!("{json}"),
                    Err(e) => {
                        eprintln!("error: failed to serialise JSON: {e}");
                        return EXIT_ERROR;
                    }
                }
            } else {
                println!("{} entries", result.len());
            }
            EXIT_SUCCESS
        }
        Command::Tui {
            left,
            right,
            ignore_whitespace,
            ignore_whitespace_and_case,
            context,
            algorithm,
            deadline,
            language,
            syntax_depth,
            width,
            max_hunks,
            max_lines_per_hunk,
        } => {
            let cfg = config::load();
            let algorithm = algorithm
                .or_else(|| {
                    cfg.defaults
                        .algorithm
                        .as_deref()
                        .and_then(config::parse_algorithm)
                })
                .unwrap_or(AlgorithmArg::Myers);
            let deadline = deadline.or(cfg.defaults.deadline);

            if algorithm == AlgorithmArg::Syntax {
                eprintln!("warning: experimental --algorithm syntax (see ROUGH_EDGES.md)");
            }

            let result = diff_core::compare_files(DiffRequest {
                left,
                right,
                mode: DiffMode::Split,
                ignore: if ignore_whitespace_and_case {
                    IgnoreMode::WhitespaceAndCase
                } else if ignore_whitespace {
                    IgnoreMode::Whitespace
                } else {
                    IgnoreMode::None
                },
                context,
                max_bytes: Some(10 * 1024 * 1024),
                algorithm: algorithm.into(),
                deadline_ms: deadline,
                language,
                syntax_depth,
            });
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    eprint_error(&e);
                    return EXIT_ERROR;
                }
            };
            let output = diff_tui::render_preview(
                &result,
                diff_tui::TuiRenderOptions {
                    width,
                    max_hunks,
                    max_lines_per_hunk,
                },
            );
            print!("{output}");
            if result.stats.additions == 0 && result.stats.deletions == 0 {
                EXIT_SUCCESS
            } else {
                EXIT_DIFFERENCES
            }
        }
        Command::Merge {
            base: _,
            ours: _,
            theirs: _,
            out: _,
        } => {
            eprintln!("error: dt merge is not implemented yet");
            EXIT_ERROR
        }
        Command::Git {
            command: GitCommand::Install,
        } => {
            eprintln!("error: dt git install is not implemented yet");
            EXIT_ERROR
        }
        Command::Office { command } => run_office(command),
        Command::Ai {
            provider,
            api_key,
            model,
            base_url,
            file,
            json,
            command,
        } => run_ai(provider, api_key, model, base_url, file, json, command),
        Command::Config { command } => run_config(command),
    }
}

/// Handle `dt ai` subcommands.
fn run_ai(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    base_url: Option<String>,
    file: Option<PathBuf>,
    json: bool,
    command: AiCommand,
) -> i32 {
    // 0. Handle commands that don't need API keys or diff input.
    if let AiCommand::ListPackages = &command {
        let packages = ai_adapter::prompts::list_prompt_packages();
        if json {
            if let Ok(j) = serde_json::to_string_pretty(&packages) {
                println!("{j}");
            }
        } else {
            println!("Available AI Prompt Packages:\n");
            for p in &packages {
                println!("  {:18} {:30} v{} ({:?})", p.id, p.name, p.version, p.category);
                println!("    {}", p.description);
                println!("    Tags: {}", p.tags.join(", "));
                println!();
            }
        }
        return EXIT_SUCCESS;
    }

    // 1. Resolve API key: flag > env var > prompt for error.
    let api_key = match api_key.or_else(|| resolve_api_key_from_env(&provider)) {
        Some(k) if !k.is_empty() => k,
        _ => {
            eprintln!("error: no API key provided. Use --api-key or set");
            let env_var = match provider.as_str() {
                "anthropic" => "ANTHROPIC_API_KEY",
                _ => "OPENAI_API_KEY",
            };
            eprintln!("       the {env_var} environment variable.");
            return EXIT_ERROR;
        }
    };

    // 2. Read diff text from file or stdin.
    let diff_text = match read_diff_text(file.as_ref()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return EXIT_ERROR;
        }
    };

    if diff_text.trim().is_empty() {
        eprintln!("error: empty diff input");
        return EXIT_ERROR;
    }

    // 3. Build BackendConfig.
    let provider_enum = match provider.as_str() {
        "anthropic" => Provider::Anthropic,
        "custom" => Provider::Custom,
        _ => Provider::OpenAI,
    };
    let model = model.unwrap_or_else(|| {
        match provider_enum {
            Provider::OpenAI => "gpt-4o-mini",
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::Custom => "",
        }
        .to_string()
    });
    let config = BackendConfig {
        provider: provider_enum,
        api_key,
        model,
        base_url,
        ..Default::default()
    };

    // 4. Create AiAdapter.
    let adapter = match AiAdapter::new(config) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: failed to create AI adapter: {e}");
            return EXIT_ERROR;
        }
    };

    // 5. Build the ChatRequest using the prompts module.
    let chat_request = match &command {
        AiCommand::ListPackages => unreachable!(),
        AiCommand::GenerateSummary { language } => ai_adapter::prompts::build_summary_request(
            adapter.config().model.as_str(),
            &diff_text,
            language,
        ),
        AiCommand::FullReview { language } => ai_adapter::prompts::build_full_review_request(
            adapter.config().model.as_str(),
            &diff_text,
            language,
        ),
        AiCommand::GenerateCommitMsg => ai_adapter::prompts::build_commit_msg_request(
            adapter.config().model.as_str(),
            &diff_text,
        ),
        AiCommand::AssessRisk {
            file_path,
            language,
        } => ai_adapter::prompts::build_risk_request(
            adapter.config().model.as_str(),
            file_path,
            &diff_text,
            language,
        ),
        AiCommand::ReviewQuestions { language } => {
            ai_adapter::prompts::build_review_questions_request(
                adapter.config().model.as_str(),
                &diff_text,
                language,
            )
        }
        AiCommand::ExplainHunk { language } => ai_adapter::prompts::build_explain_hunk_request(
            adapter.config().model.as_str(),
            &diff_text,
            language,
        ),
        AiCommand::Run {
            package,
            language,
            file_path,
        } => match ai_adapter::prompts::build_package_request(
            package,
            adapter.config().model.as_str(),
            &diff_text,
            language.as_deref(),
            file_path.as_deref(),
        ) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("error: {e}");
                return EXIT_ERROR;
            }
        },
    };

    // 6. Call the LLM.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");
    let response = match rt.block_on(adapter.complete(chat_request)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: AI request failed: {e}");
            return EXIT_ERROR;
        }
    };

    // 7 & 8. Parse and output.
    match &command {
        AiCommand::ListPackages => unreachable!(),
        AiCommand::GenerateSummary { .. } => {
            match ai_adapter::prompts::parse_summary_response(&response.content) {
                Ok(summary) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&summary) {
                            println!("{j}");
                        }
                    } else {
                        println!("Summary: {}", summary.summary);
                        if !summary.intents.is_empty() {
                            println!("Intents:");
                            for intent in &summary.intents {
                                println!("  - {intent}");
                            }
                        }
                        if !summary.risks.is_empty() {
                            println!("Risks:");
                            for risk in &summary.risks {
                                println!("  - {risk}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse AI response: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::FullReview { .. } => {
            match ai_adapter::prompts::parse_full_review_response(&response.content) {
                Ok(report) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&report) {
                            println!("{j}");
                        }
                    } else {
                        println!("=== AI Code Review Report ===\n");
                        println!("TL;DR: {}", report.summary.summary);
                        if !report.summary.intents.is_empty() {
                            println!("\nKey Intents:");
                            for intent in &report.summary.intents {
                                println!("  • {intent}");
                            }
                        }
                        let level_str = match report.risk.risk_level {
                            RiskLevel::Low => "LOW",
                            RiskLevel::Medium => "MEDIUM",
                            RiskLevel::High => "HIGH",
                        };
                        println!("\nRisk Rating: {level_str}");
                        println!("Justification: {}", report.risk.justification);
                        if !report.risk.affected_areas.is_empty() {
                            println!("Affected Areas: {}", report.risk.affected_areas.join(", "));
                        }
                        if !report.questions.is_empty() {
                            println!("\nReviewer Questions to Ask:");
                            for (i, q) in report.questions.iter().enumerate() {
                                println!("  {}. {}", i + 1, q.question);
                                if let Some(ctx) = &q.context {
                                    println!("     Context: {ctx}");
                                }
                            }
                        }
                        if let Some(commit) = &report.suggested_commit {
                            println!("\nSuggested Commit: {}", commit.subject);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse AI full review response: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::GenerateCommitMsg => {
            match ai_adapter::prompts::parse_commit_msg_response(&response.content) {
                Ok(msg) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&msg) {
                            println!("{j}");
                        }
                    } else {
                        println!("{}", msg.subject);
                        if let Some(body) = &msg.body {
                            if !body.trim().is_empty() {
                                println!();
                                println!("{body}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse AI response: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::AssessRisk { .. } => {
            match ai_adapter::prompts::parse_risk_response(&response.content) {
                Ok(risk) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&risk) {
                            println!("{j}");
                        }
                    } else {
                        let level_str = match risk.risk_level {
                            RiskLevel::Low => "LOW",
                            RiskLevel::Medium => "MEDIUM",
                            RiskLevel::High => "HIGH",
                        };
                        println!("Risk level: {level_str}");
                        println!("Justification: {}", risk.justification);
                        if !risk.affected_areas.is_empty() {
                            println!("Affected areas:");
                            for area in &risk.affected_areas {
                                println!("  - {area}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse AI response: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::ReviewQuestions { .. } => {
            match ai_adapter::prompts::parse_review_questions_response(&response.content) {
                Ok(questions) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&questions) {
                            println!("{j}");
                        }
                    } else {
                        println!("Reviewer Questions:");
                        for (i, q) in questions.iter().enumerate() {
                            println!("  {}. {}", i + 1, q.question);
                            if let Some(ctx) = &q.context {
                                println!("     Context: {ctx}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse AI review questions: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::ExplainHunk { .. } => {
            match ai_adapter::prompts::parse_explain_hunk_response(&response.content) {
                Ok(exp) => {
                    if json {
                        if let Ok(j) = serde_json::to_string_pretty(&exp) {
                            println!("{j}");
                        }
                    } else {
                        println!("Explanation:\n{}", exp.explanation);
                        if !exp.key_points.is_empty() {
                            println!("\nKey Points:");
                            for pt in &exp.key_points {
                                println!("  • {pt}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("error: failed to parse explanation: {e}");
                    if !json {
                        eprintln!("raw response: {}", response.content);
                    }
                    return EXIT_ERROR;
                }
            }
        }
        AiCommand::Run { package, .. } => {
            match package.as_str() {
                "full-review" => {
                    if let Ok(rep) = ai_adapter::prompts::parse_full_review_response(&response.content) {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&rep).unwrap());
                        } else {
                            println!("Summary: {}", rep.summary.summary);
                        }
                        return EXIT_SUCCESS;
                    }
                }
                "change-summary" | "summary" => {
                    if let Ok(sum) = ai_adapter::prompts::parse_summary_response(&response.content) {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&sum).unwrap());
                        } else {
                            println!("Summary: {}", sum.summary);
                        }
                        return EXIT_SUCCESS;
                    }
                }
                "commit-message" | "commit" => {
                    if let Ok(msg) = ai_adapter::prompts::parse_commit_msg_response(&response.content) {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&msg).unwrap());
                        } else {
                            println!("{}", msg.subject);
                        }
                        return EXIT_SUCCESS;
                    }
                }
                "risk-assessment" | "risk" => {
                    if let Ok(risk) = ai_adapter::prompts::parse_risk_response(&response.content) {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&risk).unwrap());
                        } else {
                            println!("Risk: {:?}", risk.risk_level);
                        }
                        return EXIT_SUCCESS;
                    }
                }
                "review-questions" | "questions" => {
                    if let Ok(qs) = ai_adapter::prompts::parse_review_questions_response(&response.content) {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&qs).unwrap());
                        } else {
                            for q in qs { println!("- {}", q.question); }
                        }
                        return EXIT_SUCCESS;
                    }
                }
                _ => {}
            }
            // Generic output
            println!("{}", response.content);
        }
    }

    EXIT_SUCCESS
}

/// Resolve API key from environment variable based on provider.
fn resolve_api_key_from_env(provider: &str) -> Option<String> {
    match provider {
        "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
        _ => std::env::var("OPENAI_API_KEY").ok(),
    }
}

/// Read diff text from a file or stdin.
fn read_diff_text(file: Option<&PathBuf>) -> std::io::Result<String> {
    use std::io::Read;
    match file {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

/// Handle `dt config` subcommands.
fn run_config(cmd: ConfigCommand) -> i32 {
    match cmd {
        ConfigCommand::Get => {
            let cfg = config::load();
            let path = config::config_path();
            if let Some(algo) = &cfg.defaults.algorithm {
                println!("algorithm = {algo}");
            }
            if let Some(dl) = cfg.defaults.deadline {
                println!("deadline = {dl}");
            }
            if cfg.defaults.algorithm.is_none() && cfg.defaults.deadline.is_none() {
                println!("(no config set — using built-in defaults)");
            } else if !path.exists() {
                println!("(no config file at {})", path.display());
            }
            EXIT_SUCCESS
        }
        ConfigCommand::Set { key, value } => {
            let mut cfg = config::load();
            match key.as_str() {
                "algorithm" => {
                    if config::parse_algorithm(&value).is_none() {
                        eprintln!(
                            "error: unknown algorithm '{value}'.  \
                             Valid values: myers, histogram, patience"
                        );
                        return EXIT_ERROR;
                    }
                    cfg.defaults.algorithm = Some(value);
                }
                "deadline" => {
                    let ms: u64 = match value.parse() {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!(
                                "error: deadline must be a positive integer (milliseconds): {e}"
                            );
                            return EXIT_ERROR;
                        }
                    };
                    cfg.defaults.deadline = Some(ms);
                }
                other => {
                    eprintln!(
                        "error: unknown key '{other}'.  \
                         Valid keys: algorithm, deadline"
                    );
                    return EXIT_ERROR;
                }
            }
            if let Err(e) = config::save(&cfg) {
                eprintln!("error: {e}");
                return EXIT_ERROR;
            }
            println!("Saved config to {}", config::config_path().display());
            EXIT_SUCCESS
        }
        ConfigCommand::Reset => match config::delete_config_file() {
            Ok(()) => {
                println!("Config reset (file deleted).");
                EXIT_SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                EXIT_ERROR
            }
        },
    }
}

/// Handle `dt office` subcommands.
fn run_office(cmd: OfficeCommand) -> i32 {
    match cmd {
        OfficeCommand::Diff {
            left,
            right,
            json,
            sheets,
        } => {
            let mut opts = office_adapter::OfficeDiffOptions::default();
            if !sheets.is_empty() {
                opts.sheets = sheets;
            }

            match office_adapter::compare_office_files(left, right, opts) {
                Ok(result) => {
                    // Print warnings to stderr.
                    for w in &result.warnings {
                        eprintln!("warning: {w}");
                    }

                    if json {
                        match serde_json::to_string_pretty(&result) {
                            Ok(j) => println!("{j}"),
                            Err(e) => {
                                eprintln!("error: failed to serialise JSON: {e}");
                                return EXIT_ERROR;
                            }
                        }
                    } else {
                        print_office_human_summary(&result);
                    }

                    let has_changes = result.summary.sections_total > 0
                        && (result.summary.sections_added > 0
                            || result.summary.sections_deleted > 0
                            || result.summary.sections_changed > 0);
                    if has_changes {
                        EXIT_DIFFERENCES
                    } else {
                        EXIT_SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    EXIT_ERROR
                }
            }
        }
    }
}

fn office_format_label(f: office_adapter::OfficeFormat) -> &'static str {
    match f {
        office_adapter::OfficeFormat::Word => "Word",
        office_adapter::OfficeFormat::PowerPoint => "PowerPoint",
        office_adapter::OfficeFormat::Excel => "Excel",
    }
}

fn print_office_human_summary(result: &office_adapter::OfficeDiffResult) {
    let label = office_format_label(result.format);
    let s = &result.summary;

    let mut parts: Vec<String> = Vec::new();
    if s.sections_added > 0 {
        parts.push(format!("{} added", s.sections_added));
    }
    if s.sections_deleted > 0 {
        parts.push(format!("{} deleted", s.sections_deleted));
    }
    if s.sections_changed > 0 {
        parts.push(format!("{} changed", s.sections_changed));
    }
    if s.sections_unchanged > 0 {
        parts.push(format!("{} unchanged", s.sections_unchanged));
    }

    let change_summary = if parts.is_empty() {
        "no changes".to_owned()
    } else {
        parts.join(", ")
    };

    println!(
        "{label} document: {sections_total} sections, {change_summary}",
        sections_total = s.sections_total,
    );

    if s.total_atomic_changes > 0 {
        println!("  {} atomic changes", s.total_atomic_changes);

        for section in &result.sections {
            if section.changes.is_empty() {
                continue;
            }
            for change in &section.changes {
                let kind = office_atomic_kind_label(change.kind.clone());
                let label_str = &section.label;
                match (change.before.as_deref(), change.after.as_deref()) {
                    (Some(before), Some(after)) => {
                        let diff_preview = abbreviate_diff(before, after);
                        println!("    {kind} in {label_str} ({diff_preview})");
                    }
                    (Some(before), None) => {
                        let short = abbreviate_text(before);
                        println!("    {kind} in {label_str}: {short}");
                    }
                    (None, Some(after)) => {
                        let short = abbreviate_text(after);
                        println!("    {kind} in {label_str}: {short}");
                    }
                    (None, None) => {
                        println!("    {kind} in {label_str}");
                    }
                }
            }
        }
    }

    if !result.unsupported_features.is_empty() {
        println!("  Unsupported features:");
        for f in &result.unsupported_features {
            println!("    - {f}");
        }
    }
}

fn office_atomic_kind_label(kind: office_adapter::OfficeAtomicChangeKind) -> &'static str {
    match kind {
        office_adapter::OfficeAtomicChangeKind::TextChanged => "Text changed",
        office_adapter::OfficeAtomicChangeKind::TextAdded => "Text added",
        office_adapter::OfficeAtomicChangeKind::TextDeleted => "Text deleted",
        office_adapter::OfficeAtomicChangeKind::StructureChanged => "Structure changed",
        office_adapter::OfficeAtomicChangeKind::FormattingChanged => "Formatting changed",
        office_adapter::OfficeAtomicChangeKind::ImageChanged => "Image changed",
        office_adapter::OfficeAtomicChangeKind::FormulaChanged => "Formula changed",
        office_adapter::OfficeAtomicChangeKind::SheetRenamed => "Sheet renamed",
        office_adapter::OfficeAtomicChangeKind::SlideReordered => "Slide reordered",
    }
}

/// Shorten a single text value to at most 60 chars for human output.
fn abbreviate_text(text: &str) -> String {
    let one_line = text.replace('\n', " | ");
    if one_line.len() <= 60 {
        one_line
    } else {
        format!("{}…", &one_line[..59])
    }
}

/// Make a compact "before → after" preview for human output.
fn abbreviate_diff(before: &str, after: &str) -> String {
    let b = abbreviate_text(before);
    let a = abbreviate_text(after);
    // Limit total output.
    if b.len() + a.len() <= 50 {
        format!("{b} → {a}")
    } else {
        format!("{}… → {}…", &b[..b.len().min(22)], &a[..a.len().min(22)])
    }
}

/// Result for a single file pair in a two-folder comparison.
#[derive(Debug, Serialize)]
struct PairDiffResult {
    path: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<diff_core::DiffStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<Vec<String>>,
}

/// Positional two-folder comparison using `git_adapter::compare_folders`.
/// Displays human-readable tree view with [A], [M], [D], [ ] statuses,
/// supports --json output, and exits 0 for identical, 1 for differences, 2 for errors.
fn run_compare_folders(left_root: PathBuf, right_root: PathBuf, json: bool) -> i32 {
    let diff = match git_adapter::compare_folders(&left_root, &right_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return EXIT_ERROR;
        }
    };

    let has_differences = diff.entries.iter().any(|e| {
        matches!(
            e.status,
            EntryStatus::Added | EntryStatus::Deleted | EntryStatus::Modified
        )
    });

    if json {
        match serde_json::to_string_pretty(&diff.entries) {
            Ok(j) => println!("{j}"),
            Err(e) => {
                eprintln!("error: failed to serialise JSON: {e}");
                return EXIT_ERROR;
            }
        }
    } else {
        for entry in &diff.entries {
            let tag = match entry.status {
                EntryStatus::Added => "[A]",
                EntryStatus::Modified => "[M]",
                EntryStatus::Deleted => "[D]",
                EntryStatus::Unchanged => "[ ]",
                EntryStatus::Directory => continue,
            };
            println!("{tag}  {}", entry.path.display());
        }
    }

    if has_differences {
        EXIT_DIFFERENCES
    } else {
        EXIT_SUCCESS
    }
}

/// Two-folder comparison: walk both trees, diff matching file pairs in parallel.
fn run_dir_diff(
    left_root: PathBuf,
    right_root: PathBuf,
    json: bool,
    algorithm: AlgorithmArg,
    deadline: Option<u64>,
    progress: bool,
    language: Option<String>,
    syntax_depth: Option<String>,
) -> i32 {
    if !right_root.exists() {
        eprintln!("error: {} does not exist", right_root.display());
        return EXIT_ERROR;
    }
    if !right_root.is_dir() {
        eprintln!("error: {} is not a directory", right_root.display());
        return EXIT_ERROR;
    }

    // Scan both trees.
    let left_entries = match git_adapter::scan_tree(&left_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return EXIT_ERROR;
        }
    };
    let right_entries = match git_adapter::scan_tree(&right_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return EXIT_ERROR;
        }
    };

    // Build maps: relative path (as string) → full filesystem path.
    // Only include files (not directories).
    let left_files: Vec<_> = left_entries
        .iter()
        .filter(|e| e.status != EntryStatus::Directory)
        .collect();
    let right_files: Vec<_> = right_entries
        .iter()
        .filter(|e| e.status != EntryStatus::Directory)
        .collect();

    // Collect all unique relative paths across both sides.
    let mut all_paths: Vec<&str> = left_files
        .iter()
        .map(|e| e.path.to_str().unwrap_or(""))
        .chain(right_files.iter().map(|e| e.path.to_str().unwrap_or("")))
        .collect();
    all_paths.sort();
    all_paths.dedup();

    // Diff each file pair in parallel.
    let results: Vec<PairDiffResult> = all_paths
        .par_iter()
        .map(|rel_path| {
            let left_path = left_root.join(rel_path);
            let right_path = right_root.join(rel_path);
            let left_exists = left_path.exists();
            let right_exists = right_path.exists();

            match (left_exists, right_exists) {
                (true, true) => {
                    // Both exist — diff them.
                    let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> = if progress {
                        let label = rel_path.to_string();
                        Some(Box::new(move |p| match p {
                            diff_core::Progress::Reading { path, bytes } => {
                                let kb = bytes / 1024;
                                eprintln!("  [{label}] reading {} ({} KiB)", path.display(), kb);
                            }
                            diff_core::Progress::Normalizing => {
                                eprintln!("  [{label}] normalizing...");
                            }
                            diff_core::Progress::ComputingDiff => {
                                eprintln!("  [{label}] computing diff...");
                            }
                            diff_core::Progress::ComputingIntralineSpans => {
                                eprintln!("  [{label}] computing intraline spans...");
                            }
                            diff_core::Progress::BuildingHunks => {
                                eprintln!("  [{label}] building hunks...");
                            }
                        }))
                    } else {
                        None
                    };
                    match diff_core::compare_files_with_progress(
                        DiffRequest {
                            left: left_path,
                            right: right_path,
                            mode: DiffMode::Split,
                            ignore: IgnoreMode::None,
                            context: 3,
                            max_bytes: Some(10 * 1024 * 1024),
                            algorithm: algorithm.into(),
                            deadline_ms: deadline,
                            language: language.clone(),
                            syntax_depth: syntax_depth.clone(),
                        },
                        progress_cb,
                    ) {
                        Ok(result) => {
                            let changed = result.stats.additions > 0 || result.stats.deletions > 0;
                            PairDiffResult {
                                path: rel_path.to_string(),
                                status: if changed { "modified" } else { "unchanged" },
                                stats: Some(result.stats),
                                warnings: if result.warnings.is_empty() {
                                    None
                                } else {
                                    Some(result.warnings)
                                },
                            }
                        }
                        Err(e) => PairDiffResult {
                            path: rel_path.to_string(),
                            status: "error",
                            stats: None,
                            warnings: Some(vec![e.to_string()]),
                        },
                    }
                }
                (true, false) => PairDiffResult {
                    path: rel_path.to_string(),
                    status: "deleted",
                    stats: None,
                    warnings: None,
                },
                (false, true) => PairDiffResult {
                    path: rel_path.to_string(),
                    status: "added",
                    stats: None,
                    warnings: None,
                },
                (false, false) => PairDiffResult {
                    path: rel_path.to_string(),
                    status: "error",
                    stats: None,
                    warnings: Some(vec!["file vanished between scan and diff".into()]),
                },
            }
        })
        .collect();

    // Collect results (sorted by path for deterministic output).
    let mut sorted: Vec<&PairDiffResult> = results.iter().collect();
    sorted.sort_by_key(|r| &r.path);

    if json {
        match serde_json::to_string_pretty(&sorted) {
            Ok(j) => println!("{j}"),
            Err(e) => {
                eprintln!("error: failed to serialise JSON: {e}");
                return EXIT_ERROR;
            }
        }
    } else {
        for r in &sorted {
            println!("{}  {}", r.status, r.path);
        }
    }

    EXIT_SUCCESS
}

/// Format a `DiffError` for stderr output. Exits (via return code) are
/// handled by the caller.
fn eprint_error(err: &DiffError) {
    eprintln!("error: {err}");
}
