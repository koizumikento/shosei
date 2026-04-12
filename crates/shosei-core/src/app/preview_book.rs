use crate::{
    cli_api::CommandContext,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct PreviewBookResult {
    pub summary: String,
}

pub fn preview_book(command: &CommandContext) -> Result<PreviewBookResult, RepoError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    let book = context.book.expect("selected book must exist");
    Ok(PreviewBookResult {
        summary: format!("preview is not implemented yet for {}", book.id),
    })
}
