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
        let resolved =
            config::resolve_book_config(&context).map_err(|_| RepoError::NotInitialized {
                start: book.config_path.display().to_string(),
            })?;
        let plan = pipeline::prose_build_plan(context);
        let outputs = resolved.outputs();
        let manuscript_count = resolved.manuscript_files().len();
        return Ok(BuildBookResult {
            summary: format!(
                "build plan ready for {} with {} manuscript file(s), outputs: {}, stages: {}",
                book.id,
                manuscript_count,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                },
                plan.stages.join(", ")
            ),
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}
