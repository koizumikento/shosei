use crate::domain::RepoContext;

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub context: RepoContext,
    pub stages: Vec<&'static str>,
}

pub fn prose_build_plan(context: RepoContext) -> BuildPlan {
    BuildPlan {
        context,
        stages: vec!["resolve-config", "prepare-manuscript", "invoke-pandoc"],
    }
}
