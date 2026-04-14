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
        #[arg(long, value_name = "PROFILE", value_parser = ["paper", "conference-preprint"])]
        config_profile: Option<String>,
        #[arg(long, value_name = "MODE", value_parser = ["single-book", "series"])]
        repo_mode: Option<String>,
        #[arg(long, value_name = "TITLE")]
        title: Option<String>,
        #[arg(long, value_name = "AUTHOR")]
        author: Option<String>,
        #[arg(long, value_name = "LANGUAGE")]
        language: Option<String>,
        #[arg(long, value_name = "OUTPUT", value_parser = ["kindle", "print", "both"])]
        output_preset: Option<String>,
    },
    Explain {
        #[arg(long)]
        book: Option<String>,
        #[arg(long)]
        json: bool,
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
    Story {
        #[command(subcommand)]
        command: StoryCommands,
    },
    Series {
        #[command(subcommand)]
        command: SeriesCommands,
    },
    Page {
        #[command(subcommand)]
        command: PageCommands,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
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
    Renumber {
        #[arg(long, default_value_t = 1, value_name = "NUMBER")]
        start_at: usize,
        #[arg(long, default_value_t = 2, value_name = "WIDTH")]
        width: usize,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum StoryCommands {
    Check {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Drift {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Sync {
        #[arg(long)]
        book: Option<String>,
        #[arg(long = "from", value_name = "SOURCE", value_parser = ["shared"])]
        source: Option<String>,
        #[arg(long = "to", value_name = "DESTINATION", value_parser = ["shared"])]
        destination: Option<String>,
        #[arg(long, value_name = "KIND", value_parser = ["character", "location", "term", "faction"])]
        kind: Option<String>,
        #[arg(long, value_name = "ID")]
        id: Option<String>,
        #[arg(long, value_name = "REPORT")]
        report: Option<PathBuf>,
        #[arg(long)]
        force: bool,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Map {
        #[arg(long)]
        book: Option<String>,
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
    Scaffold {
        #[arg(long)]
        shared: bool,
        #[arg(long)]
        force: bool,
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

#[derive(Debug, Subcommand)]
pub enum SeriesCommands {
    Sync {
        #[arg(long, value_name = "PATH", default_value = ".")]
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::{ChapterCommands, Cli, Commands, SeriesCommands, StoryCommands};

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
            "paper",
            "--config-profile",
            "conference-preprint",
            "--repo-mode",
            "series",
        ]);

        match cli.command {
            Commands::Init {
                path,
                config_template,
                config_profile,
                repo_mode,
                ..
            } => {
                assert_eq!(path, Some(PathBuf::from("./my-series")));
                assert_eq!(config_template.as_deref(), Some("paper"));
                assert_eq!(config_profile.as_deref(), Some("conference-preprint"));
                assert_eq!(repo_mode.as_deref(), Some("series"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_init_non_interactive_field_overrides() {
        let cli = Cli::parse_from([
            "shosei",
            "init",
            "./my-book",
            "--non-interactive",
            "--config-template",
            "novel",
            "--repo-mode",
            "single-book",
            "--title",
            "Custom Title",
            "--author",
            "Ken",
            "--language",
            "ja-JP",
            "--output-preset",
            "both",
        ]);

        match cli.command {
            Commands::Init {
                path,
                non_interactive,
                config_template,
                repo_mode,
                title,
                author,
                language,
                output_preset,
                ..
            } => {
                assert_eq!(path, Some(PathBuf::from("./my-book")));
                assert!(non_interactive);
                assert_eq!(config_template.as_deref(), Some("novel"));
                assert_eq!(repo_mode.as_deref(), Some("single-book"));
                assert_eq!(title.as_deref(), Some("Custom Title"));
                assert_eq!(author.as_deref(), Some("Ken"));
                assert_eq!(language.as_deref(), Some("ja-JP"));
                assert_eq!(output_preset.as_deref(), Some("both"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_explain_json_output_flag() {
        let cli = Cli::parse_from(["shosei", "explain", "--json", "--book", "vol-01"]);

        match cli.command {
            Commands::Explain { book, json, .. } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert!(json);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_doctor_json_output_flag() {
        let cli = Cli::parse_from(["shosei", "doctor", "--json"]);

        match cli.command {
            Commands::Doctor { json } => {
                assert!(json);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_chapter_renumber_command() {
        let cli = Cli::parse_from([
            "shosei",
            "chapter",
            "renumber",
            "--start-at",
            "3",
            "--width",
            "4",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Chapter {
                command:
                    ChapterCommands::Renumber {
                        start_at,
                        width,
                        dry_run,
                        ..
                    },
            } => {
                assert_eq!(start_at, 3);
                assert_eq!(width, 4);
                assert!(dry_run);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_scaffold_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "scaffold",
            "--shared",
            "--force",
            "--path",
            "books/vol-01",
        ]);

        match cli.command {
            Commands::Story {
                command:
                    StoryCommands::Scaffold {
                        shared,
                        force,
                        path,
                        ..
                    },
            } => {
                assert!(shared);
                assert!(force);
                assert_eq!(path, PathBuf::from("books/vol-01"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_map_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "map",
            "--book",
            "vol-01",
            "--path",
            "books/vol-01",
        ]);

        match cli.command {
            Commands::Story {
                command: StoryCommands::Map { book, path },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(path, PathBuf::from("books/vol-01"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_check_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "check",
            "--book",
            "vol-01",
            "--path",
            "books/vol-01",
        ]);

        match cli.command {
            Commands::Story {
                command: StoryCommands::Check { book, path },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(path, PathBuf::from("books/vol-01"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_drift_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "drift",
            "--book",
            "vol-01",
            "--path",
            "books/vol-01",
        ]);

        match cli.command {
            Commands::Story {
                command: StoryCommands::Drift { book, path },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(path, PathBuf::from("books/vol-01"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_sync_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "sync",
            "--book",
            "vol-01",
            "--from",
            "shared",
            "--kind",
            "character",
            "--id",
            "lead",
            "--force",
            "--path",
            "books/vol-01",
        ]);

        match cli.command {
            Commands::Story {
                command:
                    StoryCommands::Sync {
                        book,
                        source,
                        destination,
                        kind,
                        id,
                        report,
                        force,
                        path,
                    },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(source.as_deref(), Some("shared"));
                assert_eq!(destination, None);
                assert_eq!(kind.as_deref(), Some("character"));
                assert_eq!(id.as_deref(), Some("lead"));
                assert_eq!(report, None);
                assert!(force);
                assert_eq!(path, PathBuf::from("books/vol-01"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_sync_to_shared_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "sync",
            "--book",
            "vol-01",
            "--to",
            "shared",
            "--kind",
            "character",
            "--id",
            "lead",
        ]);

        match cli.command {
            Commands::Story {
                command:
                    StoryCommands::Sync {
                        book,
                        source,
                        destination,
                        kind,
                        id,
                        report,
                        force,
                        path,
                    },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(source, None);
                assert_eq!(destination.as_deref(), Some("shared"));
                assert_eq!(kind.as_deref(), Some("character"));
                assert_eq!(id.as_deref(), Some("lead"));
                assert_eq!(report, None);
                assert!(!force);
                assert_eq!(path, PathBuf::from("."));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_story_sync_report_command() {
        let cli = Cli::parse_from([
            "shosei",
            "story",
            "sync",
            "--book",
            "vol-01",
            "--from",
            "shared",
            "--report",
            "dist/reports/vol-01-story-drift.json",
            "--force",
        ]);

        match cli.command {
            Commands::Story {
                command:
                    StoryCommands::Sync {
                        book,
                        source,
                        destination,
                        kind,
                        id,
                        report,
                        force,
                        path,
                    },
            } => {
                assert_eq!(book.as_deref(), Some("vol-01"));
                assert_eq!(source.as_deref(), Some("shared"));
                assert_eq!(destination, None);
                assert_eq!(kind, None);
                assert_eq!(id, None);
                assert_eq!(
                    report,
                    Some(PathBuf::from("dist/reports/vol-01-story-drift.json"))
                );
                assert!(force);
                assert_eq!(path, PathBuf::from("."));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_series_sync_command() {
        let cli = Cli::parse_from(["shosei", "series", "sync", "--path", "repo"]);

        match cli.command {
            Commands::Series {
                command: SeriesCommands::Sync { path },
            } => {
                assert_eq!(path, PathBuf::from("repo"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }
}
