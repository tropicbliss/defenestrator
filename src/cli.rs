use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(author, about)]
pub struct Args {
    /// Name list path
    #[structopt(short, long)]
    pub path: PathBuf,

    /// Number of parallel requests
    #[structopt(short, long, default_value = "8")]
    pub parallel_requests: usize,

    /// Rate limit timeout (in seconds)
    #[structopt(short, long, default_value = "200")]
    pub timeout: u64,
}

impl Args {
    pub fn new() -> Self {
        Self::from_args()
    }
}
