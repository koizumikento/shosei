use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "shosei", version, about = "Japanese publishing workflow CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init {
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
    Build {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Validate {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Preview {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Doctor,
    Handoff {
        destination: String,
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}
