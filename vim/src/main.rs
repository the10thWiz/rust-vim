//
// basic.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::error::Error;

use vim_core::Curse;
use flexi_logger::{Logger, FileSpec};

fn main() -> Result<(), Box<dyn Error>> {
    let _tmp = Logger::try_with_env()?.log_to_file(FileSpec::try_from("./rvim.log")?).start()?;
    Curse::stdout().run()?;
    Ok(())
}
