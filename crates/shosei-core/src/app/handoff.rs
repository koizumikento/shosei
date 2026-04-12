use crate::{
    cli_api::CommandContext,
    repo::{self, RepoError},
};

#[derive(Debug, Clone)]
pub struct HandoffResult {
    pub summary: String,
}

pub fn handoff(command: &CommandContext, destination: &str) -> Result<HandoffResult, RepoError> {
    let context = repo::require_book_context(repo::discover(
        &command.start_path,
        command.book_id.as_deref(),
    )?)?;

    let book = context.book.expect("selected book must exist");
    Ok(HandoffResult {
        summary: format!(
            "handoff packaging is not implemented yet for {} ({destination})",
            book.id
        ),
    })
}
