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
        let _config =
            config::load_book_config(&book.config_path).map_err(|_| RepoError::NotInitialized {
                start: book.config_path.display().to_string(),
            })?;
        return Ok(ValidateBookResult {
            summary: format!("validation plan ready for {}", book.id),
        });
    }

    unreachable!("series repositories without a selected book are rejected")
}
