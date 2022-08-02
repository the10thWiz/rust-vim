//
// args.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::path::PathBuf;

use clap::Parser;

/// # Rust based vim variant
///
/// A complete re-write of vim in Rust
#[derive(Debug, Parser)]
#[clap(version, author, about)]
pub struct Args {
    /// Files to open
    files: Vec<PathBuf>,
    /// Open files in Read Only mode
    #[clap(short = 'R', long)]
    read_only: bool,
    /// Do not execute initialization
    #[clap(long)]
    clean: bool,
    /// Time the startup sequence
    #[clap(long)]
    time_startup: bool,
    /// Run command before starting interactive mode
    #[clap(short, long)]
    command: Vec<String>,
}

