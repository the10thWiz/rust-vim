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
    #[clap(short = 'R', long)]
    read_only: bool,
    #[clap(long)]
    clean: bool,
    #[clap(long)]
    time_startup: bool,
    #[clap(short, long)]
    command: Vec<String>,
}

