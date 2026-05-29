mod config;

use clap::{Parser, Subcommand, ValueEnum};
use diff_core::{DiffAlgorithm, DiffError, DiffMode, DiffRequest, IgnoreMode};
use git_adapter::EntryStatus;
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::process;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AlgorithmArg {
    Myers,
    Patience,
    Histogram,
}

impl From<AlgorithmArg> for DiffAlgorithm {
    fn from(arg: AlgorithmArg) -> Self {
        match arg {
            AlgorithmArg::Myers => DiffAlgorithm::Myers,
            AlgorithmArg::Patience => DiffAlgorithm::Patience,
            AlgorithmArg::Histogram => DiffAlgorithm::Histogram,
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
    },
    /// Scan a directory tree, or compare two directories.
    ///
    /// With only ROOT: lists all files in the tree (like `ls -R`).
    /// With --right: diffs matching file pairs across left and right roots in
    /// parallel, reporting additions, deletions, and modifications.
    Dir {
        root: PathBuf,
        /// Right-hand directory for two-folder comparison.  File pairs with
        /// matching relative paths are diffed in parallel using all available
        /// CPU cores.
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
    /// Manage persistent CLI configuration (~/.config/diff-tool/config.toml).
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
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
        Command::Diff { left, right, json, ignore_whitespace, ignore_whitespace_and_case, context, algorithm, deadline, progress } => {
            // Merge config defaults: CLI flags > config > hardcoded defaults.
            let cfg = config::load();
            let algorithm = algorithm
                .or_else(|| {
                    cfg.defaults.algorithm
                        .as_deref()
                        .and_then(config::parse_algorithm)
                })
                .unwrap_or(AlgorithmArg::Myers);
            let deadline = deadline.or(cfg.defaults.deadline);

            let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> =
                if progress {
                    Some(Box::new(|p| {
                        match p {
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
        Command::Dir { root, right, json, algorithm, deadline, progress } => {
            if !root.exists() {
                eprintln!("error: {} does not exist", root.display());
                return EXIT_ERROR;
            }
            if !root.is_dir() {
                eprintln!("error: {} is not a directory", root.display());
                return EXIT_ERROR;
            }

            // Merge config defaults for algorithm and deadline.
            let cfg = config::load();
            let algorithm = algorithm
                .or_else(|| {
                    cfg.defaults.algorithm
                        .as_deref()
                        .and_then(config::parse_algorithm)
                })
                .unwrap_or(AlgorithmArg::Myers);
            let deadline = deadline.or(cfg.defaults.deadline);

            if let Some(right_root) = right {
                return run_dir_diff(root, right_root, json, algorithm, deadline, progress);
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
        Command::Config { command } => run_config(command),
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
                            eprintln!("error: deadline must be a positive integer (milliseconds): {e}");
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
        ConfigCommand::Reset => {
            match config::delete_config_file() {
                Ok(()) => {
                    println!("Config reset (file deleted).");
                    EXIT_SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    EXIT_ERROR
                }
            }
        }
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

/// Two-folder comparison: walk both trees, diff matching file pairs in parallel.
fn run_dir_diff(
    left_root: PathBuf,
    right_root: PathBuf,
    json: bool,
    algorithm: AlgorithmArg,
    deadline: Option<u64>,
    progress: bool,
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
                    let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> =
                        if progress {
                            let label = rel_path.to_string();
                            Some(Box::new(move |p| {
                                match p {
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
                                }
                            }))
                        } else {
                            None
                        };
                    match diff_core::compare_files_with_progress(DiffRequest {
                        left: left_path,
                        right: right_path,
                        mode: DiffMode::Split,
                        ignore: IgnoreMode::None,
                        context: 3,
                        max_bytes: Some(10 * 1024 * 1024),
                        algorithm: algorithm.into(),
                        deadline_ms: deadline,
                    }, progress_cb) {
                        Ok(result) => {
                            let changed =
                                result.stats.additions > 0 || result.stats.deletions > 0;
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
                    warnings: Some(vec![
                        "file vanished between scan and diff".into()
                    ]),
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
