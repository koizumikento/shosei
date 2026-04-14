mod args;
mod exit_code;
mod output;
mod prompts;

use anyhow::Result;
use clap::Parser;
use shosei_core::{app, cli_api::CommandContext};

use crate::args::{ChapterCommands, Cli, Commands, PageCommands, SeriesCommands, StoryCommands};

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
            let wizard_answers = if non_interactive || config_template.is_some() {
                None
            } else {
                Some(prompts::prompt_init_wizard()?)
            };
            let result = app::init_project(app::InitProjectOptions {
                root: target,
                non_interactive,
                force,
                config_template: wizard_answers
                    .as_ref()
                    .map(|answers| answers.config_template.clone())
                    .or(config_template),
                repo_mode: wizard_answers
                    .as_ref()
                    .map(|answers| answers.repo_mode.clone()),
                title: wizard_answers.as_ref().map(|answers| answers.title.clone()),
                author: wizard_answers
                    .as_ref()
                    .map(|answers| answers.author.clone()),
                language: wizard_answers
                    .as_ref()
                    .map(|answers| answers.language.clone()),
                output_preset: wizard_answers
                    .as_ref()
                    .map(|answers| answers.output_preset.clone()),
            })?;
            output::print_line(&result.summary);
            if wizard_answers.as_ref().map(|answers| answers.run_doctor) == Some(true) {
                let doctor = app::doctor();
                output::print_line(&doctor.summary);
            }
            Ok(exit_code::OK)
        }
        Commands::Explain { book, path } => {
            let result = app::explain_config(&CommandContext::new(path, book, None))?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Build { book, target, path } => {
            let result = app::build_book(&CommandContext::new(path, book, target))?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
        Commands::Validate { book, target, path } => {
            let result = app::validate_book(&CommandContext::new(path, book, target))?;
            output::print_line(&result.summary);
            Ok(if result.has_errors {
                exit_code::FAILURE
            } else {
                exit_code::OK
            })
        }
        Commands::Preview {
            book,
            target,
            watch,
            path,
        } => {
            let command = CommandContext::new(path, book, target);
            if watch {
                app::watch_preview(&command, output::print_line)?;
            } else {
                let result = app::preview_book(&command)?;
                output::print_line(&result.summary);
            }
            Ok(exit_code::OK)
        }
        Commands::Chapter { command } => match command {
            ChapterCommands::Add {
                chapter_path,
                title,
                before,
                after,
                book,
                path,
            } => {
                let result = app::chapter_add(
                    &CommandContext::new(path, book, None),
                    app::ChapterAddOptions {
                        chapter_path,
                        title,
                        before,
                        after,
                    },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
            ChapterCommands::Move {
                chapter_path,
                before,
                after,
                book,
                path,
            } => {
                let result = app::chapter_move(
                    &CommandContext::new(path, book, None),
                    app::ChapterMoveOptions {
                        chapter_path,
                        before,
                        after,
                    },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
            ChapterCommands::Remove {
                chapter_path,
                delete_file,
                book,
                path,
            } => {
                let result = app::chapter_remove(
                    &CommandContext::new(path, book, None),
                    app::ChapterRemoveOptions {
                        chapter_path,
                        delete_file,
                    },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
            ChapterCommands::Renumber {
                start_at,
                width,
                dry_run,
                book,
                path,
            } => {
                let result = app::chapter_renumber(
                    &CommandContext::new(path, book, None),
                    app::ChapterRenumberOptions {
                        start_at,
                        width,
                        dry_run,
                    },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
        },
        Commands::Story { command } => match command {
            StoryCommands::Check { book, path } => {
                let result = app::story_check(
                    &CommandContext::new(path, book, None),
                    app::StoryCheckOptions {},
                )?;
                output::print_line(&result.summary);
                Ok(if result.has_errors {
                    exit_code::FAILURE
                } else {
                    exit_code::OK
                })
            }
            StoryCommands::Drift { book, path } => {
                let result = app::story_drift(
                    &CommandContext::new(path, book, None),
                    app::StoryDriftOptions {},
                )?;
                output::print_line(&result.summary);
                Ok(if result.has_errors {
                    exit_code::FAILURE
                } else {
                    exit_code::OK
                })
            }
            StoryCommands::Sync {
                book,
                source,
                destination,
                kind,
                id,
                report,
                force,
                path,
            } => {
                let result = app::story_sync(
                    &CommandContext::new(path, book, None),
                    app::StorySyncOptions {
                        source,
                        destination,
                        kind,
                        id,
                        report,
                        force,
                    },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
            StoryCommands::Map { book, path } => {
                let result = app::story_map(
                    &CommandContext::new(path, book, None),
                    app::StoryMapOptions {},
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
            StoryCommands::Scaffold {
                shared,
                force,
                book,
                path,
            } => {
                let result = app::story_scaffold(
                    &CommandContext::new(path, book, None),
                    app::StoryScaffoldOptions { shared, force },
                )?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
        },
        Commands::Series { command } => match command {
            SeriesCommands::Sync { path } => {
                let result = app::series_sync(&CommandContext::new(path, None, None))?;
                output::print_line(&result.summary);
                Ok(exit_code::OK)
            }
        },
        Commands::Page { command } => match command {
            PageCommands::Check { book, path } => {
                let result = app::page_check(&CommandContext::new(path, book, None))?;
                output::print_line(&result.summary);
                Ok(if result.has_errors {
                    exit_code::FAILURE
                } else {
                    exit_code::OK
                })
            }
        },
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
            let result = app::handoff(&CommandContext::new(path, book, None), &destination)?;
            output::print_line(&result.summary);
            Ok(exit_code::OK)
        }
    }
}
