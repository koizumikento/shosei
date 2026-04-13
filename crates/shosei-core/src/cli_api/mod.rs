use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub start_path: PathBuf,
    pub book_id: Option<String>,
    pub output_target: Option<String>,
}

impl CommandContext {
    pub fn new(
        start_path: impl Into<PathBuf>,
        book_id: Option<String>,
        output_target: Option<String>,
    ) -> Self {
        Self {
            start_path: start_path.into(),
            book_id,
            output_target,
        }
    }
}
