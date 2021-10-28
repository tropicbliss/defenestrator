use std::path::PathBuf;

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(author, about)]
pub struct Opt {
    /// Input file
    #[structopt(parse(from_os_str), global = true)]
    pub input: PathBuf,

    /// Set number of parallel requests
    #[structopt(short, long, default_value = "8")]
    pub parallel_requests: usize,

    /// Set rate limit timeout (in seconds)
    #[structopt(short, long, default_value = "200")]
    pub timeout: u64,
}

impl Opt {
    pub fn new() -> Self {
        Self::from_args()
    }
}
