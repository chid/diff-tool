use anyhow::Result;
use clap::{Parser, Subcommand};
use diff_core::{DiffMode, DiffRequest, IgnoreMode};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "dt", about = "Diff Tool command-line integration")]
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

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Diff { left, right, json } => {
            let result = diff_core::compare_files(DiffRequest {
                left,
                right,
                mode: DiffMode::Split,
                ignore: IgnoreMode::None,
                context: 3,
                max_bytes: Some(10 * 1024 * 1024),
            })?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "{} additions, {} deletions, {} unchanged",
                    result.stats.additions, result.stats.deletions, result.stats.unchanged
                );
            }
        }
        Command::Dir { root, json } => {
            let result = git_adapter::scan_tree(&root)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{} entries", result.len());
            }
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
        }
        Command::Git {
            command: GitCommand::Install,
        } => {
            println!("git difftool/mergetool install is not implemented yet");
        }
    }
    Ok(())
}
