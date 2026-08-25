use crate::AiTimelineProjection;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiPromptReceipt {
    thread_id: String,
    active_turn_id: String,
    after_turn_count: i32,
}

impl AiPromptReceipt {
    pub fn new(thread_id: String, active_turn_id: String, after_turn_count: i32) -> Self {
        Self {
            thread_id,
            active_turn_id,
            after_turn_count,
        }
    }

    pub fn thread_id(&self) -> &str {
        self.thread_id.as_str()
    }

    pub fn is_accepted_by(&self, active_thread_id: &str, timeline: &AiTimelineProjection) -> bool {
        if active_thread_id != self.thread_id {
            return false;
        }
        if self.active_turn_id.is_empty() {
            timeline.total_turn_count > self.after_turn_count || !timeline.active_turn_id.is_empty()
        } else {
            !timeline.active_turn_id.is_empty() && timeline.active_turn_id != self.active_turn_id
        }
    }
}
