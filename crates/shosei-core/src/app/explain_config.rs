use crate::{
    cli_api::CommandContext,
    config::{self, ExplainedConfig},
    domain::RepoMode,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct ExplainConfigResult {
    pub summary: String,
    pub explained: ExplainedConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum ExplainConfigError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Config(#[from] config::ConfigError),
}

pub fn explain_config(command: &CommandContext) -> Result<ExplainConfigResult, ExplainConfigError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;
    let explained = config::explain_book_config(&context)?;
    let book = context.book.as_ref().expect("selected book must exist");
    let mode = match context.mode {
        RepoMode::SingleBook => "single-book",
        RepoMode::Series => "series",
    };

    let mut lines = vec![
        format!("explain for {}", book.id),
        format!("repo mode: {mode}"),
        format!("repo root: {}", context.repo_root.display()),
        format!("book root: {}", book.root.display()),
        format!("config path: {}", book.config_path.display()),
    ];

    let outputs = explained.resolved.outputs();
    lines.push(format!(
        "effective outputs: {}",
        if outputs.is_empty() {
            "none".to_string()
        } else {
            outputs.join(", ")
        }
    ));
    lines.push("".to_string());
    lines.push("resolved values:".to_string());
    for value in &explained.values {
        lines.push(format!(
            "- {} = {} [{}]",
            value.field, value.value, value.origin
        ));
    }

    if context.mode == RepoMode::Series {
        lines.push("".to_string());
        lines.push("shared search paths:".to_string());
        lines.push(format!(
            "- assets = {}",
            display_repo_paths(&explained.resolved.shared.assets)
        ));
        lines.push(format!(
            "- styles = {}",
            display_repo_paths(&explained.resolved.shared.styles)
        ));
        lines.push(format!(
            "- fonts = {}",
            display_repo_paths(&explained.resolved.shared.fonts)
        ));
        lines.push(format!(
            "- metadata = {}",
            display_repo_paths(&explained.resolved.shared.metadata)
        ));
    }

    Ok(ExplainConfigResult {
        summary: lines.join("\n"),
        explained,
    })
}

fn display_repo_paths(paths: &[crate::domain::RepoPath]) -> String {
    if paths.is_empty() {
        "none".to_string()
    } else {
        paths
            .iter()
            .map(|path| path.as_str().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
