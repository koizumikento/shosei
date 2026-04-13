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
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, value_name = "TEMPLATE")]
        config_template: Option<String>,
        #[arg(long, value_name = "MODE", value_parser = ["single-book", "series"])]
        repo_mode: Option<String>,
    },
    Explain {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Build {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "TARGET", value_parser = ["kindle", "print"])]
        target: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Validate {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "TARGET", value_parser = ["kindle", "print"])]
        target: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Preview {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "TARGET", value_parser = ["kindle", "print"])]
        target: Option<String>,
        #[arg(long)]
        watch: bool,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Chapter {
        #[command(subcommand)]
        command: ChapterCommands,
    },
    Page {
        #[command(subcommand)]
        command: PageCommands,
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

#[derive(Debug, Subcommand)]
pub enum ChapterCommands {
    Add {
        #[arg(value_name = "CHAPTER_PATH")]
        chapter_path: String,
        #[arg(long, value_name = "TITLE")]
        title: Option<String>,
        #[arg(long, value_name = "CHAPTER_PATH")]
        before: Option<String>,
        #[arg(long, value_name = "CHAPTER_PATH")]
        after: Option<String>,
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Move {
        #[arg(value_name = "CHAPTER_PATH")]
        chapter_path: String,
        #[arg(long, value_name = "CHAPTER_PATH")]
        before: Option<String>,
        #[arg(long, value_name = "CHAPTER_PATH")]
        after: Option<String>,
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Remove {
        #[arg(value_name = "CHAPTER_PATH")]
        chapter_path: String,
        #[arg(long)]
        delete_file: bool,
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum PageCommands {
    Check {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::{ChapterCommands, Cli, Commands};

    #[test]
    fn parses_chapter_add_command() {
        let cli = Cli::parse_from([
            "shosei",
            "chapter",
            "add",
            "manuscript/03-new.md",
            "--title",
            "New Chapter",
            "--after",
            "manuscript/02-old.md",
        ]);

        match cli.command {
            Commands::Chapter {
                command:
                    ChapterCommands::Add {
                        chapter_path,
                        title,
                        after,
                        ..
                    },
            } => {
                assert_eq!(chapter_path, "manuscript/03-new.md");
                assert_eq!(title.as_deref(), Some("New Chapter"));
                assert_eq!(after.as_deref(), Some("manuscript/02-old.md"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_chapter_remove_command() {
        let cli = Cli::parse_from([
            "shosei",
            "chapter",
            "remove",
            "manuscript/02-old.md",
            "--delete-file",
        ]);

        match cli.command {
            Commands::Chapter {
                command:
                    ChapterCommands::Remove {
                        chapter_path,
                        delete_file,
                        ..
                    },
            } => {
                assert_eq!(chapter_path, "manuscript/02-old.md");
                assert!(delete_file);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_init_repo_mode_override() {
        let cli = Cli::parse_from([
            "shosei",
            "init",
            "./my-series",
            "--config-template",
            "business",
            "--repo-mode",
            "series",
        ]);

        match cli.command {
            Commands::Init {
                path,
                config_template,
                repo_mode,
                ..
            } => {
                assert_eq!(path, Some(PathBuf::from("./my-series")));
                assert_eq!(config_template.as_deref(), Some("business"));
                assert_eq!(repo_mode.as_deref(), Some("series"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
