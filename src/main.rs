#![warn(clippy::pedantic)]

mod cli;
mod executor;
mod utils;

use ansi_term::Colour::Red;
use anyhow::{Context, Result};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::new();
    let names = utils::get_names(&args.input)
        .context(format!("Failed to get names from {}", args.input.display()))?;
    let name_validity_data =
        utils::get_name_validity(names).context("Failed to get name validity data")?;
    let available_names = executor::run(
        name_validity_data.valid_names,
        args.parallel_requests,
        args.delay,
    )
    .await
    .context("Failed to run executor")?;
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut handle = io::BufWriter::new(handle);
    writeln!(handle)?;
    if available_names.is_empty() {
        writeln!(handle, "{}", Red.paint("No available names found"))?;
    } else {
        writeln!(handle, "Available name(s): {:?}", available_names)?;
    }
    if !name_validity_data.invalid_names.is_empty() {
        writeln!(
            handle,
            "Invalid name(s): {:?}",
            name_validity_data.invalid_names
        )?;
    }
    handle.flush()?;
    Ok(())
}
