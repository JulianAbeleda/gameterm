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
}

impl VisualComposeRuntimeState {
    pub(crate) fn new() -> Self {
        Self {
            phase: VisualComposePhase::Idle,
            history: Vec::new(),
            last_prompt: None,
            last_reply: None,
        }
    }

    fn push_message(&mut self, role: VisualComposeRole, text: String) {
        self.push_message_with_speaker(role, text, None);
    }

    fn push_message_with_speaker(
        &mut self,
        role: VisualComposeRole,
        text: String,
        speaker: Option<String>,
    ) {
        if text.trim().is_empty() {
            return;
        }
        const MAX_COMPOSE_HISTORY: usize = 20;
        self.history.push(VisualComposeMessage {
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
        self.push_message(VisualComposeRole::User, prompt.to_string());
    }

    pub(crate) fn mark_succeeded(&mut self, speaker: &str, reply: &str) {
        self.set_phase_and_history(VisualComposePhase::Succeeded);
        self.last_reply = Some(reply.to_string());
        let speaker = if speaker.trim().is_empty() {
            "Codex".to_string()
        } else {
            speaker.trim().to_string()
        };
        self.push_message_with_speaker(
            VisualComposeRole::Assistant,
            reply.to_string(),
            Some(speaker),
        );
    }

    pub(crate) fn mark_failed(&mut self, reason: &str) {
        self.set_phase_and_history(VisualComposePhase::Failed);
        self.last_reply = Some(reason.to_string());
        self.push_message(VisualComposeRole::Error, reason.to_string());
    }
}
