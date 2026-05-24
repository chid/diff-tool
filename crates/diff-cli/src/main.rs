use clap::{Parser, Subcommand};
use diff_core::{DiffError, DiffMode, DiffRequest, IgnoreMode};
use std::path::PathBuf;
use std::process;

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
    },
    Dir {
        root: PathBuf,
        #[arg(long)]
        json: bool,
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
        Command::Diff { left, right, json, ignore_whitespace, ignore_whitespace_and_case, context } => {
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
            });
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
        Command::Dir { root, json } => {
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
            base,
            ours,
            theirs,
            out,
        } => {
            println!(
                "merge requested: base={} ours={} theirs={} out={}",
                base.display(),
                ours.display(),
                theirs.display(),
                out.display()
            );
            EXIT_SUCCESS
        }
        Command::Git {
            command: GitCommand::Install,
        } => {
            println!("git difftool/mergetool install is not implemented yet");
            EXIT_SUCCESS
        }
    }
}

/// Format a `DiffError` for stderr output. Exits (via return code) are
/// handled by the caller.
fn eprint_error(err: &DiffError) {
    eprintln!("error: {err}");
}
