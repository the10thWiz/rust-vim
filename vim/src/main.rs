//
// basic.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//
use vim_core::{Result, Curse};

fn main() -> Result<()> {
    env_logger::init();
    Curse::stdout().run()
}
