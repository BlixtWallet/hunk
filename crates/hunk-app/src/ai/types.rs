use std::ops::Range;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiPromptSkillReference {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiComposerSkillBinding {
    pub token: String,
    pub range: Range<usize>,
    pub reference: AiPromptSkillReference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiPendingSteer {
    pub thread_id: String,
    pub turn_id: String,
    pub prompt: String,
    pub local_images: Vec<PathBuf>,
    pub selected_skills: Vec<AiPromptSkillReference>,
    pub skill_bindings: Vec<AiComposerSkillBinding>,
    pub accepted_after_sequence: u64,
    pub started_at: Instant,
}
