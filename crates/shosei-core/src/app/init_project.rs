use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct InitProjectResult {
    pub root: PathBuf,
    pub summary: String,
}

pub fn init_project(root: PathBuf) -> InitProjectResult {
    InitProjectResult {
        summary: format!(
            "interactive init is not implemented yet; target root would be {}",
            root.display()
        ),
        root,
    }
}
