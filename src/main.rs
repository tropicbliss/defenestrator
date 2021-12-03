#![warn(clippy::pedantic)]

mod cli;
mod executor;
mod utils;

use anyhow::{Context, Result};
use console::style;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Opt::new();
    let names = utils::get_names(&args.input)
        .with_context(|| format!("Failed to get names from {}", args.input.display()))?;
    let name_validity_data =
        utils::get_name_validity(names).with_context(|| "Failed to get name validity data")?;
    let available_names = executor::run(name_validity_data.valid_names, args.parallel_requests)
        .await
        .with_context(|| "Failed to run executor")?;
    writeln!(stdout())?;
    if available_names.is_empty() {
        writeln!(stdout(), "{}", style("No available names found").red())?;
    } else {
        writeln!(stdout(), "Available name(s): {:?}", available_names)?;
    }
    if !name_validity_data.invalid_names.is_empty() {
        writeln!(
            stdout(),
            "Invalid name(s): {:?}",
            name_validity_data.invalid_names
        )?;
    }
    Ok(())
}
