use crate::{
    cli_api::CommandContext,
    config, pipeline,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct BuildBookResult {
    pub summary: String,
}

pub fn build_book(command: &CommandContext) -> Result<BuildBookResult, RepoError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    if let Some(book) = context.book.clone() {
        let _config =
            config::load_book_config(&book.config_path).map_err(|_| RepoError::NotInitialized {
                start: book.config_path.display().to_string(),
            })?;
        let plan = pipeline::prose_build_plan(context);
        return Ok(BuildBookResult {
            summary: format!(
                "build plan ready for {} with stages: {}",
                book.id,
                plan.stages.join(", ")
            ),
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}
