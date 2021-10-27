#![warn(clippy::pedantic)]

mod cli;
mod executor;
mod utils;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = cli::Args::new();
    let names = utils::get_names(args.path)?;
    let name_validity_data = utils::get_name_validity(&names);
    let available_names = executor::run(
        name_validity_data.valid_names,
        args.parallel_requests,
        args.timeout,
    )
    .await?;
    println!();
    println!("Available username(s): {:?}", available_names);
    println!(
        "Invalid username(s): {:?}",
        name_validity_data.invalid_names
    );
    Ok(())
}
