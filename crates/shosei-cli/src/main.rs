mod args;
mod exit_code;
mod output;
mod prompts;

use anyhow::Result;
use clap::Parser;
use shosei_core::{app, cli_api::CommandContext};

use crate::args::{Cli, Commands};

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            exit_code::FAILURE
        }
    };
    std::process::exit(code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            path,
            non_interactive,
            force,
            config_template,
        } => {
            output::print_line(prompts::init_mode_banner());
            let target = path.unwrap_or(std::env::current_dir()?);
            let result = app::init_project(app::InitProjectOptions {
                root: target,
                non_interactive,
                force,
                config_template,
            })?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Build { book, path } => {
            let result = app::build_book(&CommandContext::new(path, book))?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Validate { book, path } => {
            let result = app::validate_book(&CommandContext::new(path, book))?;
            output::print_line(&result.summary);
            Ok(if result.has_errors {
                exit_code::FAILURE
            } else {
                exit_code::OK
            })
        }
        Commands::Preview { book, path } => {
            let result = app::preview_book(&CommandContext::new(path, book))?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Doctor => {
            let result = app::doctor();
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Handoff {
            destination,
            book,
            path,
        } => {
            let result = app::handoff(&CommandContext::new(path, book), &destination)?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
    }
}
