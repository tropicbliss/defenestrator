use clap::Parser;
use std::{num::NonZeroUsize, path::PathBuf};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    /// Input file
    #[clap(parse(from_os_str), global = true)]
    pub input: PathBuf,

    /// Number of parallel requests
    #[clap(short, long, default_value = "27")]
    parallel_requests: NonZeroUsize,

    /// Rate limit delay in seconds
    #[clap(short, long, default_value = "200")]
    pub delay: u64,
}

impl Args {
    pub fn new() -> Self {
        Self::parse()
    }

    pub fn get_parallel_requests(&self) -> usize {
        self.parallel_requests.into()
    }
}
