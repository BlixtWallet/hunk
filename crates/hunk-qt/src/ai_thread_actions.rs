use crate::AiThreadCatalogProjection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiThreadActionKind {
    Create,
    Select,
    Fork,
    Archive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiThreadActionReceipt {
    kind: AiThreadActionKind,
    thread_id: String,
    started_thread_id: String,
}

impl AiThreadActionReceipt {
    pub fn create() -> Self {
        Self::new(AiThreadActionKind::Create, String::new())
    }

    pub fn select(thread_id: String) -> Self {
        Self::new(AiThreadActionKind::Select, thread_id)
    }

    pub fn fork(thread_id: String) -> Self {
        Self::new(AiThreadActionKind::Fork, thread_id)
    }

    pub fn archive(thread_id: String) -> Self {
        Self::new(AiThreadActionKind::Archive, thread_id)
    }

    pub fn record_started_thread(&mut self, thread_id: String) -> bool {
        if !matches!(
            self.kind,
            AiThreadActionKind::Create | AiThreadActionKind::Fork
        ) {
            return false;
        }
        self.started_thread_id = thread_id;
        true
    }

    pub fn is_complete(&self, projection: &AiThreadCatalogProjection) -> bool {
        match self.kind {
            AiThreadActionKind::Create | AiThreadActionKind::Fork => {
                !self.started_thread_id.is_empty()
                    && projection.active_thread_id == self.started_thread_id
            }
            AiThreadActionKind::Select => projection.active_thread_id == self.thread_id,
            AiThreadActionKind::Archive => {
                projection.active_thread_id != self.thread_id
                    && !projection
                        .items
                        .iter()
                        .any(|thread| thread.thread_id == self.thread_id)
            }
        }
    }

    pub fn completion_message(&self) -> &'static str {
        match self.kind {
            AiThreadActionKind::Create => "Created a new Codex thread.",
            AiThreadActionKind::Select => "Opened the Codex thread.",
            AiThreadActionKind::Fork => "Forked the Codex thread.",
            AiThreadActionKind::Archive => "Archived the Codex thread.",
        }
    }

    fn new(kind: AiThreadActionKind, thread_id: String) -> Self {
        Self {
            kind,
            thread_id,
            started_thread_id: String::new(),
        }
    }
}
