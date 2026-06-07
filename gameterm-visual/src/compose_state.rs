#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisualComposePhase {
    Idle,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VisualComposeRole {
    User,
    Assistant,
    System,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisualComposeMessage {
    pub(crate) turn_id: u64,
    pub(crate) block_index: usize,
    pub(crate) role: VisualComposeRole,
    pub(crate) text: String,
    pub(crate) speaker: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct VisualComposeRuntimeState {
    pub(crate) phase: VisualComposePhase,
    pub(crate) history: Vec<VisualComposeMessage>,
    pub(crate) last_prompt: Option<String>,
    pub(crate) last_reply: Option<String>,
    next_turn_id: u64,
    active_turn_id: Option<u64>,
    active_block_index: usize,
}

impl VisualComposeRuntimeState {
    pub(crate) fn new() -> Self {
        Self {
            phase: VisualComposePhase::Idle,
            history: Vec::new(),
            last_prompt: None,
            last_reply: None,
            next_turn_id: 1,
            active_turn_id: None,
            active_block_index: 0,
        }
    }

    fn push_message(&mut self, role: VisualComposeRole, text: String) {
        let turn_id = self.active_or_new_turn_id();
        self.push_message_with_speaker(role, text, None, turn_id);
    }

    fn push_message_with_speaker(
        &mut self,
        role: VisualComposeRole,
        text: String,
        speaker: Option<String>,
        turn_id: u64,
    ) {
        if text.trim().is_empty() {
            return;
        }
        const MAX_COMPOSE_HISTORY: usize = 20;
        let block_index = self.next_block_index_for_turn(turn_id);
        self.history.push(VisualComposeMessage {
            turn_id,
            block_index,
            role,
            text,
            speaker,
        });
        if self.history.len() > MAX_COMPOSE_HISTORY {
            let excess = self.history.len() - MAX_COMPOSE_HISTORY;
            self.history.drain(0..excess);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.phase = VisualComposePhase::Idle;
        self.history.clear();
        self.last_prompt = None;
        self.last_reply = None;
        self.active_turn_id = None;
        self.active_block_index = 0;
    }

    pub(crate) fn latest_assistant_speaker(&self) -> Option<&str> {
        self.history.iter().rev().find_map(|message| {
            if matches!(
                message.role,
                VisualComposeRole::Assistant | VisualComposeRole::System
            ) {
                message.speaker.as_deref()
            } else {
                None
            }
        })
    }

    fn set_phase_and_history(&mut self, phase: VisualComposePhase) {
        self.phase = phase;
    }

    pub(crate) fn mark_running(&mut self, prompt: &str) {
        self.set_phase_and_history(VisualComposePhase::Running);
        self.last_prompt = Some(prompt.to_string());
        self.last_reply = None;
        let turn_id = self.allocate_turn_id();
        self.active_turn_id = Some(turn_id);
        self.active_block_index = 0;
        self.push_message_with_speaker(VisualComposeRole::User, prompt.to_string(), None, turn_id);
    }

    pub(crate) fn mark_succeeded(&mut self, speaker: &str, reply: &str) {
        self.set_phase_and_history(VisualComposePhase::Succeeded);
        self.last_reply = Some(reply.to_string());
        let speaker = if speaker.trim().is_empty() {
            "Codex".to_string()
        } else {
            speaker.trim().to_string()
        };
        let turn_id = self.active_or_new_turn_id();
        self.push_message_with_speaker(
            VisualComposeRole::Assistant,
            reply.to_string(),
            Some(speaker),
            turn_id,
        );
    }

    pub(crate) fn mark_failed(&mut self, reason: &str) {
        self.set_phase_and_history(VisualComposePhase::Failed);
        self.last_reply = Some(reason.to_string());
        self.push_message(VisualComposeRole::Error, reason.to_string());
    }

    fn allocate_turn_id(&mut self) -> u64 {
        let turn_id = self.next_turn_id;
        self.next_turn_id = self.next_turn_id.saturating_add(1);
        turn_id
    }

    fn active_or_new_turn_id(&mut self) -> u64 {
        if let Some(turn_id) = self.active_turn_id {
            turn_id
        } else {
            let turn_id = self.allocate_turn_id();
            self.active_turn_id = Some(turn_id);
            self.active_block_index = 0;
            turn_id
        }
    }

    fn next_block_index_for_turn(&mut self, turn_id: u64) -> usize {
        if self.active_turn_id != Some(turn_id) {
            self.active_turn_id = Some(turn_id);
            self.active_block_index = 0;
        }
        let block_index = self.active_block_index;
        self.active_block_index = self.active_block_index.saturating_add(1);
        block_index
    }
}
