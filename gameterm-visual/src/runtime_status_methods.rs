use std::path::Path;

use crate::{RunCommandTarget, SceneRuntime, VisualComposeRole};

impl SceneRuntime {
    pub fn mark_action_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.bump_generation();
    }

    pub fn mark_compose_running(&mut self, status: impl Into<String>, prompt: &str) {
        let prompt = prompt.trim().to_string();
        self.compose_state.mark_running(&prompt);
        self.status = status.into();
        self.record_runtime_event("compose", format!("submit: {prompt}"));
        self.bump_generation();
    }

    pub fn mark_compose_succeeded(&mut self, speaker: &str, reply: &str) {
        let trimmed_reply = reply.trim();
        let role = if speaker.trim().is_empty() {
            VisualComposeRole::Assistant
        } else if speaker.eq_ignore_ascii_case("scene") {
            VisualComposeRole::System
        } else if speaker.eq_ignore_ascii_case("error") {
            VisualComposeRole::Error
        } else {
            VisualComposeRole::Assistant
        };
        let event_detail = if trimmed_reply.is_empty() {
            format!("{:?}: {speaker}: <empty>", role)
        } else {
            format!("{:?}: {speaker}: {trimmed_reply}", role)
        };
        self.compose_state.mark_succeeded(speaker, trimmed_reply);
        self.record_runtime_event("compose", event_detail);
        self.bump_generation();
    }

    pub fn mark_compose_succeeded_blocks(
        &mut self,
        speaker: &str,
        blocks: &[String],
        reveal_all: bool,
    ) -> Vec<(u64, usize)> {
        let role = if speaker.trim().is_empty() {
            VisualComposeRole::Assistant
        } else if speaker.eq_ignore_ascii_case("scene") {
            VisualComposeRole::System
        } else if speaker.eq_ignore_ascii_case("error") {
            VisualComposeRole::Error
        } else {
            VisualComposeRole::Assistant
        };
        let event_detail = if blocks.is_empty() {
            format!("{:?}: {speaker}: <empty>", role)
        } else {
            format!("{:?}: {speaker}: {} block(s)", role, blocks.len())
        };
        let ids = self
            .compose_state
            .mark_succeeded_blocks(speaker, blocks, reveal_all);
        self.record_runtime_event("compose", event_detail);
        self.bump_generation();
        ids
    }

    pub fn mark_compose_block_speaking(&mut self, turn_id: u64, block_index: usize) {
        if self.compose_state.mark_block_speaking(turn_id, block_index) {
            self.record_runtime_event(
                "compose",
                format!("speaking block: turn={turn_id} block={block_index}"),
            );
            self.bump_generation();
        }
    }

    pub fn mark_compose_block_done(&mut self, turn_id: u64, block_index: usize) {
        if self.compose_state.mark_block_done(turn_id, block_index) {
            self.record_runtime_event(
                "compose",
                format!("done block: turn={turn_id} block={block_index}"),
            );
            self.bump_generation();
        }
    }

    pub fn clear_compose_history(&mut self) {
        self.compose_state.clear();
        self.record_runtime_event("compose", "cleared transcript");
        self.bump_generation();
    }

    pub fn mark_compose_failed(&mut self, error: &str) {
        let trimmed_error = error.trim();
        let event_detail = if trimmed_error.is_empty() {
            "failed".to_string()
        } else {
            format!("failed: {trimmed_error}")
        };
        self.compose_state.mark_failed(trimmed_error);
        self.record_runtime_event("compose", event_detail);
        self.bump_generation();
    }

    pub fn mark_open_file_dispatched(&mut self, path: &Path) {
        self.status = format!("OpenFile opening: {}", path.display());
        self.bump_generation();
    }

    pub fn mark_run_command_spawning(&mut self, argv: &[String], target: RunCommandTarget) {
        self.status = format!("RunCommand opening {}: {}", target.as_str(), argv.join(" "));
        self.bump_generation();
    }

    pub fn mark_run_command_spawned(
        &mut self,
        argv: &[String],
        target: RunCommandTarget,
        pane_id: usize,
    ) {
        self.status = format!(
            "RunCommand opened {} pane {pane_id}: {}",
            target.as_str(),
            argv.join(" ")
        );
        self.bump_generation();
    }

    pub fn mark_run_command_failed(
        &mut self,
        argv: &[String],
        target: RunCommandTarget,
        error: impl std::fmt::Display,
    ) {
        self.status = format!(
            "RunCommand failed ({}): {}: {error}",
            target.as_str(),
            argv.join(" ")
        );
        self.bump_generation();
    }
}
