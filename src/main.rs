#![warn(clippy::pedantic)]

mod cli;
mod executor;
mod utils;

use anyhow::Result;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Opt::new();
    let names = utils::get_names(args.input)?;
    let name_validity_data = utils::get_name_validity(&names);
    let available_names = executor::run(
        name_validity_data.valid_names,
        args.parallel_requests,
        args.timeout,
    )
    .await?;
    writeln!(stdout())?;
    writeln!(stdout(), "Available username(s): {:?}", available_names)?;
    writeln!(
        stdout(),
        "Invalid username(s): {:?}",
        name_validity_data.invalid_names
    )?;
    Ok(())
}
