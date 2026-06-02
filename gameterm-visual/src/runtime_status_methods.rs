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
        self.compose_state.mark_succeeded(trimmed_reply);
        self.record_runtime_event("compose", event_detail);
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
