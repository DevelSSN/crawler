use clap::Parser;
use std::path::PathBuf;
use url::Url;

#[derive(Parser, Debug, Clone)]
#[command(version = option_env!("PROJECT_VERSION").unwrap_or("unknown"))]
pub struct Args {
    #[arg(long)]
    pub index: String,

    #[arg(long)]
    pub output: PathBuf,

    #[arg(long, default_value_t = 3)]
    pub depth: usize,

    #[arg(long, default_value_t = 4)]
    pub workers: usize,

    #[arg(long, default_value = "false")]
    pub hardcode_external: bool,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub url: Url,
    pub depth: usize,
}
