//! CLI definition: thin arg parsing; the binary shim calls into this.

use clap::Parser;

/// Pretty, searchable Neovim and tmux cheatsheets for Omarchy.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Open directly on one sheet (e.g. "nvim", "tmux").
    #[arg(long)]
    pub section: Option<String>,

    /// Print the available sheet names and exit (scriptable).
    #[arg(long)]
    pub list_sections: bool,
}
