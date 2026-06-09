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
    pub(crate) visibility: VisualComposeVisibility,
    pub(crate) revealed_chars: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VisualComposeVisibility {
    Queued,
    Speaking,
    Done,
}

impl VisualComposeVisibility {
    pub(crate) fn is_visible(self) -> bool {
        // TTS is an enhancement layer, not the source of truth for dialogue.
        // Keep queued blocks visible so a delayed/stalled voice worker cannot
        // make a completed compose reply disappear from the transcript.
        matches!(self, Self::Queued | Self::Speaking | Self::Done)
    }
}

impl VisualComposeMessage {
    pub(crate) fn rendered_text(&self) -> String {
        match self.revealed_chars {
            Some(limit) => self.text.chars().take(limit).collect(),
            None => self.text.clone(),
        }
    }

    fn is_revealable(&self) -> bool {
        matches!(
            (&self.role, self.visibility),
            (
                VisualComposeRole::Assistant | VisualComposeRole::System,
                VisualComposeVisibility::Queued | VisualComposeVisibility::Speaking
            )
        )
    }

    fn reveal_len(&self) -> usize {
        self.text.chars().count()
    }

    fn reveal_all(&mut self) -> bool {
        if self.revealed_chars.is_none() {
            return false;
        }
        self.revealed_chars = None;
        true
    }

    fn reveal_next_chunk(&mut self, chunk_chars: usize) -> bool {
        let Some(current) = self.revealed_chars else {
            return false;
        };
        let total = self.reveal_len();
        if current >= total {
            self.revealed_chars = None;
            return false;
        }
        let next = next_fake_stream_reveal_len(&self.text, current, chunk_chars);
        if next >= total {
            self.revealed_chars = None;
        } else {
            self.revealed_chars = Some(next);
        }
        next > current
    }
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
        let _ = self.push_message_with_speaker(
            role,
            text,
            None,
            turn_id,
            VisualComposeVisibility::Done,
            None,
        );
    }

    fn push_message_with_speaker(
        &mut self,
        role: VisualComposeRole,
        text: String,
        speaker: Option<String>,
        turn_id: u64,
        visibility: VisualComposeVisibility,
        revealed_chars: Option<usize>,
    ) -> Option<(u64, usize)> {
        if text.trim().is_empty() {
            return None;
        }
        const MAX_COMPOSE_HISTORY: usize = 20;
        let block_index = self.next_block_index_for_turn(turn_id);
        self.history.push(VisualComposeMessage {
            turn_id,
            block_index,
            role,
            text,
            speaker,
            visibility,
            revealed_chars,
        });
        if self.history.len() > MAX_COMPOSE_HISTORY {
            let excess = self.history.len() - MAX_COMPOSE_HISTORY;
            self.history.drain(0..excess);
        }
        Some((turn_id, block_index))
    }

    pub(crate) fn clear(&mut self) {
        self.phase = VisualComposePhase::Idle;
        self.history.clear();
        self.last_prompt = None;
        self.last_reply = None;
        self.active_turn_id = None;
        self.active_block_index = 0;
    }

    pub(crate) fn backend_prompt_with_context(&self, prompt: &str) -> String {
        let prompt = prompt.trim();
        if self.history.is_empty() {
            return prompt.to_string();
        }

        const RECENT_CONTEXT_TURNS: usize = 6;
        const CONTEXT_MESSAGE_MAX_CHARS: usize = 500;
        let recent_turns =
            self.recent_turn_context_summary(RECENT_CONTEXT_TURNS, CONTEXT_MESSAGE_MAX_CHARS);
        if recent_turns.trim().is_empty() {
            return prompt.to_string();
        }

        format!(
            "GameTerm Scene Mode conversation context follows. Use it for continuity across Scene Mode turns. If the latest user prompt is a fragment, answer it as a continuation of the recent turns. Do not say you lack prior context when the recent turns provide it.\n\nLatest user prompt:\n{prompt}\n\nRecent turns:\n{recent_turns}\n\nResponse rules:\n- Answer the latest user prompt.\n- Use recent turns as background, not as a topic switch.\n- Keep useful technical details visible when they matter.\n- Return only the assistant reply text unless a structured Scene Mode JSON patch is specifically needed."
        )
    }

    fn recent_turn_context_summary(&self, max_turns: usize, max_message_chars: usize) -> String {
        let mut turn_ids = self
            .history
            .iter()
            .map(|message| message.turn_id)
            .collect::<Vec<_>>();
        turn_ids.dedup();
        if turn_ids.len() > max_turns {
            turn_ids = turn_ids.split_off(turn_ids.len() - max_turns);
        }

        turn_ids
            .into_iter()
            .filter_map(|turn_id| {
                let mut lines = Vec::new();
                for message in self
                    .history
                    .iter()
                    .filter(|message| message.turn_id == turn_id)
                {
                    let text = cap_compose_context_text(&message.text, max_message_chars);
                    if text.is_empty() {
                        continue;
                    }
                    let label = match message.role {
                        VisualComposeRole::User => "User".to_string(),
                        VisualComposeRole::Assistant => message
                            .speaker
                            .as_deref()
                            .filter(|speaker| !speaker.trim().is_empty())
                            .unwrap_or("Assistant")
                            .to_string(),
                        VisualComposeRole::System => "Scene".to_string(),
                        VisualComposeRole::Error => "Error".to_string(),
                    };
                    lines.push(format!("{label}: {text}"));
                }
                if lines.is_empty() {
                    None
                } else {
                    Some(lines.join("\n"))
                }
            })
            .collect::<Vec<_>>()
            .join("\n\n")
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
        self.reveal_all_pending();
        self.set_phase_and_history(VisualComposePhase::Running);
        self.last_prompt = Some(prompt.to_string());
        self.last_reply = None;
        let turn_id = self.allocate_turn_id();
        self.active_turn_id = Some(turn_id);
        self.active_block_index = 0;
        let _ = self.push_message_with_speaker(
            VisualComposeRole::User,
            prompt.to_string(),
            None,
            turn_id,
            VisualComposeVisibility::Done,
            None,
        );
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
        let _ = self.push_message_with_speaker(
            VisualComposeRole::Assistant,
            reply.to_string(),
            Some(speaker),
            turn_id,
            VisualComposeVisibility::Done,
            None,
        );
    }

    pub(crate) fn mark_succeeded_blocks(
        &mut self,
        speaker: &str,
        blocks: &[String],
        reveal_all: bool,
    ) -> Vec<(u64, usize)> {
        self.set_phase_and_history(VisualComposePhase::Succeeded);
        self.last_reply = Some(blocks.join("\n\n"));
        let speaker = if speaker.trim().is_empty() {
            "Codex".to_string()
        } else {
            speaker.trim().to_string()
        };
        let turn_id = self.active_or_new_turn_id();
        let visibility = if reveal_all {
            VisualComposeVisibility::Done
        } else {
            VisualComposeVisibility::Queued
        };
        blocks
            .iter()
            .filter_map(|block| {
                self.push_message_with_speaker(
                    VisualComposeRole::Assistant,
                    block.to_string(),
                    Some(speaker.clone()),
                    turn_id,
                    visibility,
                    if reveal_all { None } else { Some(0) },
                )
            })
            .collect()
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

    pub(crate) fn mark_block_speaking(&mut self, turn_id: u64, block_index: usize) -> bool {
        let visibility_changed =
            self.set_block_visibility(turn_id, block_index, VisualComposeVisibility::Speaking);
        let reveal_changed = self.reveal_block_next_chunk(turn_id, block_index, 24);
        visibility_changed || reveal_changed
    }

    pub(crate) fn mark_block_done(&mut self, turn_id: u64, block_index: usize) -> bool {
        let visibility_changed =
            self.set_block_visibility(turn_id, block_index, VisualComposeVisibility::Done);
        let reveal_changed = self.reveal_block_all(turn_id, block_index);
        visibility_changed || reveal_changed
    }

    pub(crate) fn advance_reveal(&mut self, chunk_chars: usize) -> bool {
        for message in &mut self.history {
            if !message.is_revealable() || message.revealed_chars.is_none() {
                continue;
            }
            return message.reveal_next_chunk(chunk_chars.max(1));
        }
        false
    }

    fn reveal_all_pending(&mut self) -> bool {
        let mut changed = false;
        for message in &mut self.history {
            changed |= message.reveal_all();
        }
        changed
    }

    fn reveal_block_next_chunk(
        &mut self,
        turn_id: u64,
        block_index: usize,
        chunk_chars: usize,
    ) -> bool {
        let Some(message) = self
            .history
            .iter_mut()
            .find(|message| message.turn_id == turn_id && message.block_index == block_index)
        else {
            return false;
        };
        message.reveal_next_chunk(chunk_chars.max(1))
    }

    fn reveal_block_all(&mut self, turn_id: u64, block_index: usize) -> bool {
        let Some(message) = self
            .history
            .iter_mut()
            .find(|message| message.turn_id == turn_id && message.block_index == block_index)
        else {
            return false;
        };
        message.reveal_all()
    }

    fn set_block_visibility(
        &mut self,
        turn_id: u64,
        block_index: usize,
        visibility: VisualComposeVisibility,
    ) -> bool {
        let Some(message) = self
            .history
            .iter_mut()
            .find(|message| message.turn_id == turn_id && message.block_index == block_index)
        else {
            return false;
        };
        if matches!(message.role, VisualComposeRole::User) {
            return false;
        }
        message.visibility = visibility;
        true
    }
}

fn cap_compose_context_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut capped = normalized.chars().take(max_chars).collect::<String>();
    capped.push_str("...");
    capped
}

fn next_fake_stream_reveal_len(text: &str, current_chars: usize, target_chars: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    if current_chars >= total {
        return total;
    }

    let target_chars = target_chars.max(1);
    let min_boundary_width = (target_chars / 3).max(6);
    let mut sentence_boundary = None;
    let mut word_boundary = None;
    let mut width = 0usize;

    for index in current_chars..total {
        width = width.saturating_add(1);
        let next = index + 1;
        if chars[index] == '\n' && chars.get(index + 1) == Some(&'\n') {
            return (next + 1).min(total);
        }
        if width >= min_boundary_width && matches!(chars[index], '.' | '!' | '?' | ':') {
            sentence_boundary = Some(next);
        }
        if width >= min_boundary_width && chars[index].is_whitespace() {
            word_boundary = Some(next);
        }
        if width >= target_chars {
            return sentence_boundary
                .or(word_boundary)
                .unwrap_or(next)
                .min(total);
        }
    }

    total
}
