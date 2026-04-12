use crate::{
    cli_api::CommandContext,
    config,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct ValidateBookResult {
    pub summary: String,
}

pub fn validate_book(command: &CommandContext) -> Result<ValidateBookResult, RepoError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    if let Some(book) = &context.book {
        let resolved =
            config::resolve_book_config(&context).map_err(|_| RepoError::NotInitialized {
                start: book.config_path.display().to_string(),
            })?;
        let outputs = resolved.outputs();
        return Ok(ValidateBookResult {
            summary: format!(
                "validation plan ready for {} with outputs: {}",
                book.id,
                if outputs.is_empty() {
                    "none".to_string()
                } else {
                    outputs.join(", ")
                }
            ),
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}
