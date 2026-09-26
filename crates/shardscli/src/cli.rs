use std::path::PathBuf;

use clap::Parser;

/// The Shards shell, with selectable language syntax.
#[derive(Parser, Debug)]
#[command(name = "shards", version)]
pub struct ShardsArgs {
    /// Run a command instead of starting the prompt.
    #[arg(short = 'c', long, conflicts_with = "file")]
    pub command: Option<String>,

    /// Force the interactive prompt.
    #[arg(short, long)]
    pub interactive: bool,

    /// Initial language: rust, sh, or julia.
    #[arg(long, default_value = "rust", value_parser = ["rust", "sh", "julia"])]
    pub lang: String,

    /// Read commands from a file. Use `-` for standard input.
    pub file: Option<PathBuf>,
}
