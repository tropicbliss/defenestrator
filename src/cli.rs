use std::{num::NonZeroUsize, path::PathBuf};

use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(author, about)]
pub struct Opt {
    /// Input file
    #[structopt(parse(from_os_str), global = true)]
    pub input: PathBuf,

    /// Number of parallel requests
    #[structopt(short, long, default_value = "27")]
    pub parallel_requests: NonZeroUsize,
}

impl Opt {
    pub fn new() -> Self {
        Self::from_args()
    }
}
