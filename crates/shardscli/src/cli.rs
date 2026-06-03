use std::path::PathBuf;

use clap::Parser;

/// A Shell to oxidize your terminal
#[derive(Parser, Debug)]
#[clap(
    name = "Rushi",
    author = "Focus172",
    version = "0.1.0",
    about = "A Shell to oxidize your terminal",
    long_about = None,
)]
pub struct RushiArgs {
    /// Enables debug mode
    #[arg(short = 'd', long, default_value_t = false)]
    pub debug: bool,

    /// File to use for debug output
    #[arg(short = 'o', long, value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
    pub debug_file: Option<PathBuf>,

    /// Commands to be executed in place of interactive shell.
    #[arg(short, long, value_name = "COMMAND")]
    // pub commnad: Option<Vec<String>>,
    pub commnad: Option<String>,

    /// Commands to execute after the shell's config has been read.
    #[arg(short = 'C', long, value_name = "COMMAND")]
    pub init_command: Option<String>,

    /// Whether no-config is set. default is false.
    #[arg(short = 'N', long, default_value_t = false)]
    no_config: bool,

    /// Custom config file to use. default is ~/.config/rushi/config.rsi
    #[arg(short = 'p', long, value_name = "PATH", value_hint = clap::ValueHint::FilePath)]
    custom_config: Option<PathBuf>,

    /// Whether no-exec is set.
    #[arg(short = 'n', long, default_value_t = false)]
    no_execute: bool,

    /// Whether this is a login shell.
    #[arg(short = 'l', long, default_value_t = false)]
    is_login: bool,

    /// Whether this is an interactive session.
    #[arg(short = 'i', long = "interactive", default_value_t = false)]
    is_interactive_session: bool,

    /// Whether to enable private mode.
    #[arg(short = 'P', long = "private", default_value_t = false)]
    private_mode: bool,
    // /// Profile to login as
    // #[arg(short = 'p', long, value_name = "PROFILE")]
    // profile: Option<Profile>,

    // /// Unstable features to enable
    // #[arg(short = 'f', long, value_name = "FEATURES")]
    // features: Option<Vec<Feature>>,
}

impl RushiArgs {
    pub fn gen() -> Self {
        Self::parse().imply_args()
    }

    fn imply_args(mut self) -> Self {
        // if the first argument starts with a dash, we are a login shell
        if std::env::args().take(1).any(|arg| arg.starts_with('-')) {
            self.is_login = true;
        }

        // no_config implies private mode
        if self.no_config {
            self.private_mode = true;
        }

        // an output file implies something to output
        if self.debug_file.is_some() {
            self.debug = true;
        }

        // We are an interactive session if we have not been given an explicit
        // command or file to execute and stdin is a tty. Note that the -i or
        // --interactive options also force interactive mode.
        if self.init_command.is_none() && crossterm::tty::IsTty::is_tty(&std::io::stdin()) {
            self.is_interactive_session = true;
        }

        self
    }
}
