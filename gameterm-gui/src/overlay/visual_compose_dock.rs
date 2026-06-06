use super::clip_text;
use gameterm_visual::VnOverlayRect;
use termwiz::input::KeyCode;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneComposeDock {
    pub(super) buffer: String,
    pub(super) cursor: usize,
    pub(super) history: Vec<String>,
    history_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SceneComposeAction {
    Consumed,
    Submitted(String),
    Fallthrough,
}

impl SceneComposeDock {
    pub(super) fn handle_key(&mut self, key: KeyCode) -> SceneComposeAction {
        match key {
            KeyCode::Backspace => {
                self.remove_before_cursor();
                SceneComposeAction::Consumed
            }
            KeyCode::Delete => {
                if self.buffer.is_empty() {
                    SceneComposeAction::Fallthrough
                } else {
                    self.clear_buffer();
                    SceneComposeAction::Consumed
                }
            }
            KeyCode::LeftArrow => {
                self.cursor = self.cursor.saturating_sub(1);
                SceneComposeAction::Consumed
            }
            KeyCode::RightArrow => {
                self.cursor = (self.cursor + 1).min(self.buffer_char_len());
                SceneComposeAction::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                SceneComposeAction::Consumed
            }
            KeyCode::End => {
                self.cursor = self.buffer_char_len();
                SceneComposeAction::Consumed
            }
            KeyCode::UpArrow => {
                self.recall_previous_history();
                SceneComposeAction::Consumed
            }
            KeyCode::DownArrow => {
                self.recall_next_history();
                SceneComposeAction::Consumed
            }
            KeyCode::Enter => {
                let submitted = self.buffer.trim().to_string();
                if submitted.is_empty() {
                    SceneComposeAction::Fallthrough
                } else {
                    SceneComposeAction::Submitted(submitted)
                }
            }
            KeyCode::Char(ch) if is_compose_char(ch) => {
                self.insert_char(ch);
                SceneComposeAction::Consumed
            }
            _ => SceneComposeAction::Fallthrough,
        }
    }

    pub(super) fn mark_submitted(&mut self, prompt: &str) {
        self.history.push(prompt.to_string());
        if self.history.len() > 20 {
            self.history.remove(0);
        }
        self.clear_buffer();
    }

    pub(super) fn insert_transcript(&mut self, transcript: &str) {
        let transcript = transcript.trim();
        if transcript.is_empty() {
            return;
        }
        if !self.buffer.is_empty()
            && self.cursor == self.buffer_char_len()
            && !self.buffer.chars().last().is_some_and(char::is_whitespace)
        {
            self.insert_char(' ');
        }
        for ch in transcript.chars().filter(|ch| is_compose_char(*ch)) {
            self.insert_char(ch);
        }
    }

    pub(super) fn render_line(&self, cols: usize) -> String {
        let mut line = String::from(" Compose: ");
        line.push_str(&self.buffer_with_cursor());
        if self.buffer.is_empty() {
            line.push_str("  type here; enter submits");
        }
        clip_text(&line, cols.max(1))
    }

    pub(super) fn render_staged_dock_line(&self, cols: usize, rect: VnOverlayRect) -> String {
        let mut line = String::from(" ");
        line.push_str(&self.buffer_with_cursor());
        if self.buffer.is_empty() {
            line.push_str(" type here; enter submits");
        }
        let content_width = rect.width.min(cols.saturating_sub(rect.col)).max(1);
        let indent = " ".repeat(rect.col.min(cols.saturating_sub(1)));
        format!(
            "{indent}{:<content_width$}",
            clip_text(&line, content_width)
        )
    }

    pub(super) fn render_staged_nameplate_line(&self, cols: usize, rect: VnOverlayRect) -> String {
        let content_width = rect.width.min(cols.saturating_sub(rect.col)).max(1);
        let indent = " ".repeat(rect.col.min(cols.saturating_sub(1)));
        let label = format!("{:<content_width$}", clip_text("Composer", content_width));
        format!(
            "{indent}{:<content_width$}",
            clip_text(&label, content_width)
        )
    }

    fn insert_char(&mut self, ch: char) {
        let byte_idx = self.cursor_byte_idx();
        self.buffer.insert(byte_idx, ch);
        self.cursor += 1;
        self.history_index = None;
    }

    fn remove_before_cursor(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let start = self.cursor_to_byte_idx(self.cursor - 1);
        let end = self.cursor_byte_idx();
        self.buffer.replace_range(start..end, "");
        self.cursor -= 1;
        self.history_index = None;
    }

    fn clear_buffer(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.history_index = None;
    }

    fn recall_previous_history(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_index = match self.history_index {
            Some(index) => index.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.set_history_index(next_index);
    }

    fn recall_next_history(&mut self) {
        let Some(index) = self.history_index else {
            return;
        };
        if index + 1 >= self.history.len() {
            self.clear_buffer();
        } else {
            self.set_history_index(index + 1);
        }
    }

    fn set_history_index(&mut self, index: usize) {
        self.history_index = Some(index);
        self.buffer = self.history[index].clone();
        self.cursor = self.buffer_char_len();
    }

    fn buffer_with_cursor(&self) -> String {
        let mut out = String::new();
        for (idx, ch) in self.buffer.chars().enumerate() {
            if idx == self.cursor {
                out.push('_');
            }
            out.push(ch);
        }
        if self.cursor >= self.buffer_char_len() {
            out.push('_');
        }
        out
    }

    fn cursor_byte_idx(&self) -> usize {
        self.cursor_to_byte_idx(self.cursor)
    }

    fn cursor_to_byte_idx(&self, cursor: usize) -> usize {
        self.buffer
            .char_indices()
            .nth(cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.buffer.len())
    }

    fn buffer_char_len(&self) -> usize {
        self.buffer.chars().count()
    }
}

fn is_compose_char(ch: char) -> bool {
    !ch.is_control()
}
