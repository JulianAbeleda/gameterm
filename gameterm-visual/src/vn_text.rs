use crate::runtime_status::{clip_text, wrap_text};
use crate::{VisualComposeMessage, VisualComposeRole};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualDialogueTextBlock {
    pub kind: VisualDialogueTextBlockKind,
    pub marker: Option<String>,
    pub display_text: String,
}

impl VisualDialogueTextBlock {
    pub fn speech_allowed(&self) -> bool {
        matches!(
            self.kind,
            VisualDialogueTextBlockKind::Prose
                | VisualDialogueTextBlockKind::Heading
                | VisualDialogueTextBlockKind::Bullet
                | VisualDialogueTextBlockKind::Numbered
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualDialogueTextBlockKind {
    Blank,
    Prose,
    Heading,
    Bullet,
    Numbered,
    TechnicalSkipped,
}

pub(crate) fn wrap_compose_transcript_for_vn(
    messages: &[VisualComposeMessage],
    dialogue_width: usize,
) -> Vec<String> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for message in messages {
        if !message.visibility.is_visible() {
            continue;
        }
        let message_lines = wrap_compose_message_for_vn(message, dialogue_width);
        if message_lines.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(message_lines);
    }
    lines
}

fn wrap_compose_message_for_vn(
    message: &VisualComposeMessage,
    dialogue_width: usize,
) -> Vec<String> {
    let text = message.rendered_text();
    if text.trim().is_empty() {
        return Vec::new();
    }
    match message.role {
        VisualComposeRole::User => wrap_user_prompt_for_vn(&text, dialogue_width),
        VisualComposeRole::Assistant | VisualComposeRole::System => {
            wrap_dialogue_display_text(&text, dialogue_width)
        }
        VisualComposeRole::Error => wrap_error_for_vn(&text, dialogue_width),
    }
}

pub(crate) fn wrap_user_prompt_for_vn(prompt: &str, dialogue_width: usize) -> Vec<String> {
    let prompt_width = dialogue_width.saturating_sub(2).max(1);
    let mut lines = wrap_text(prompt, prompt_width);
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("> {line}")
            } else {
                format!("  {line}")
            }
        })
        .collect()
}

fn wrap_error_for_vn(error: &str, dialogue_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in wrap_dialogue_display_text(error, dialogue_width.saturating_sub(7).max(1)) {
        if line.is_empty() {
            lines.push(line);
        } else {
            lines.push(format!("Error: {line}"));
        }
    }
    lines
}

fn wrap_dialogue_display_text(text: &str, dialogue_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for block in dialogue_text_blocks(text) {
        match block.kind {
            VisualDialogueTextBlockKind::Blank => {
                push_blank_once(&mut lines);
            }
            VisualDialogueTextBlockKind::Heading => {
                push_blank_once(&mut lines);
                lines.extend(wrap_text(&block.display_text, dialogue_width));
                lines.push(String::new());
            }
            VisualDialogueTextBlockKind::Numbered | VisualDialogueTextBlockKind::Bullet => {
                lines.extend(wrap_marked_line(
                    block.marker.as_deref().unwrap_or("-"),
                    &block.display_text,
                    dialogue_width,
                ));
            }
            VisualDialogueTextBlockKind::Prose | VisualDialogueTextBlockKind::TechnicalSkipped => {
                lines.extend(wrap_text(
                    &strip_inline_markers(&block.display_text),
                    dialogue_width,
                ));
            }
        }
    }

    lines = collapse_blanks_between_list_items(lines);
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

pub fn dialogue_text_blocks(text: &str) -> Vec<VisualDialogueTextBlock> {
    let display_text = structured_dialogue_text(text).unwrap_or_else(|| text.trim().to_string());
    let normalized = normalize_dialogue_blocks(&display_text);
    let mut blocks = Vec::new();
    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            blocks.push(VisualDialogueTextBlock {
                kind: VisualDialogueTextBlockKind::Blank,
                marker: None,
                display_text: String::new(),
            });
            continue;
        }
        if let Some(heading) = colon_heading(line) {
            blocks.push(VisualDialogueTextBlock {
                kind: VisualDialogueTextBlockKind::Heading,
                marker: None,
                display_text: heading,
            });
            continue;
        }
        if let Some((marker, body)) = numbered_parts(line) {
            blocks.push(VisualDialogueTextBlock {
                kind: VisualDialogueTextBlockKind::Numbered,
                marker: Some(marker.to_string()),
                display_text: body.to_string(),
            });
            continue;
        }
        if let Some(body) = bullet_body(line) {
            blocks.push(VisualDialogueTextBlock {
                kind: VisualDialogueTextBlockKind::Bullet,
                marker: Some("-".to_string()),
                display_text: body.to_string(),
            });
            continue;
        }
        blocks.push(VisualDialogueTextBlock {
            kind: if dialogue_line_is_technical(line) {
                VisualDialogueTextBlockKind::TechnicalSkipped
            } else {
                VisualDialogueTextBlockKind::Prose
            },
            marker: None,
            display_text: line.to_string(),
        });
    }
    blocks
}

fn structured_dialogue_text(text: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            value
                .get("patch")
                .and_then(|patch| patch.get("dialogue"))
                .and_then(|dialogue| dialogue.get("text"))
                .and_then(serde_json::Value::as_str)
        })
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_dialogue_blocks(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let text = insert_section_breaks(&replace_display_urls(&text));
    let mut output = Vec::new();
    for line in text.lines() {
        let stripped = line.trim();
        if let Some(header) = stripped.strip_prefix("### ") {
            push_blank_once(&mut output);
            output.push(strip_inline_markers(header));
            output.push(String::new());
        } else if let Some(header) = stripped.strip_prefix("## ") {
            push_blank_once(&mut output);
            output.push(strip_inline_markers(header));
            output.push(String::new());
        } else if let Some(header) = stripped.strip_prefix("# ") {
            push_blank_once(&mut output);
            output.push(strip_inline_markers(header));
            output.push(String::new());
        } else {
            output.extend(split_flattened_list_line(line));
        }
    }
    output.join("\n")
}

fn insert_section_breaks(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if idx > 0 && starts_structured_marker(&text[idx..]) && should_break_before(text, idx) {
            output.push('\n');
        }
        output.push(ch);
    }
    output
}

fn should_break_before(text: &str, idx: usize) -> bool {
    if text[..idx].ends_with('\n') {
        return false;
    }
    text[..idx]
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .is_some_and(|ch| matches!(ch, '.' | '!' | '?' | ':' | '|'))
}

fn starts_structured_marker(text: &str) -> bool {
    starts_numbered_marker(text) || starts_bullet_marker(text)
}

fn starts_numbered_marker(text: &str) -> bool {
    let text = text
        .strip_prefix("**")
        .or_else(|| text.strip_prefix("__"))
        .unwrap_or(text);
    numbered_parts(text).is_some()
}

fn starts_bullet_marker(text: &str) -> bool {
    bullet_body(text).is_some()
}

fn numbered_parts(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    let line = line
        .strip_prefix("**")
        .or_else(|| line.strip_prefix("__"))
        .unwrap_or(line);
    for separator in [". ", ") "] {
        let Some((marker, body)) = line.split_once(separator) else {
            continue;
        };
        if marker.chars().all(|ch| ch.is_ascii_digit()) {
            return Some((marker, body));
        }
    }
    None
}

fn bullet_body(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
        .or_else(|| line.strip_prefix("• "))
}

fn wrap_marked_line(marker: &str, body: &str, dialogue_width: usize) -> Vec<String> {
    let prefix = if marker == "-" {
        "- ".to_string()
    } else {
        format!("{marker}. ")
    };
    let body_width = dialogue_width.saturating_sub(prefix.chars().count()).max(1);
    let body = strip_inline_markers(body);
    let wrapped = wrap_text(&body, body_width);
    if wrapped.is_empty() {
        return vec![prefix.trim_end().to_string()];
    }
    wrapped
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                format!("{prefix}{line}")
            } else {
                format!("{}{line}", " ".repeat(prefix.chars().count()))
            }
        })
        .collect()
}

fn strip_inline_markers(text: &str) -> String {
    text.replace("**", "").replace("__", "").replace('`', "")
}

fn colon_heading(line: &str) -> Option<String> {
    let line = strip_inline_markers(line.trim()).trim().to_string();
    let heading = line.strip_suffix(':')?.trim();
    if heading.is_empty()
        || heading.len() > 48
        || heading.contains("://")
        || heading.contains('/')
        || heading.contains('\\')
        || heading.contains('=')
        || heading.contains('@')
        || heading
            .chars()
            .any(|ch| matches!(ch, '{' | '}' | '[' | ']'))
        || heading.split_whitespace().count() > 6
    {
        return None;
    }
    let lower = heading.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "status" | "error" | "warning" | "thread" | "http" | "https"
    ) || looks_like_clock_label(heading)
    {
        return None;
    }
    heading
        .chars()
        .all(|ch| ch.is_alphanumeric() || matches!(ch, ' ' | '-' | '_' | '\''))
        .then(|| format!("{heading}:"))
}

fn looks_like_clock_label(text: &str) -> bool {
    let Some((hour, minute)) = text.split_once(':') else {
        return false;
    };
    hour.chars().all(|ch| ch.is_ascii_digit()) && minute.chars().all(|ch| ch.is_ascii_digit())
}

fn dialogue_line_is_technical(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    line.starts_with("diff --git")
        || line.starts_with("@@")
        || line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with('{')
        || line.starts_with('}')
        || line.starts_with('[') && line.ends_with(']')
        || lower.starts_with("error:")
        || lower.starts_with("warning:")
        || lower.starts_with("thread '")
        || lower.contains("stack backtrace")
        || line_is_path_or_identifier_heavy(line)
}

fn line_is_path_or_identifier_heavy(line: &str) -> bool {
    let path_like = (line.matches('/').count() >= 2 || line.matches('\\').count() >= 2)
        && line.split_whitespace().count() <= 4;
    let total_chars = line.chars().count().max(1);
    let identifier_chars = line
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '\\' | '.' | ':'))
        .count();
    let identifier_heavy = identifier_chars * 100 / total_chars > 85
        && line.split_whitespace().count() <= 4
        && !line.contains(' ');
    let punctuation_heavy =
        line.chars().filter(|ch| ch.is_ascii_punctuation()).count() * 100 / total_chars > 45;

    path_like || identifier_heavy || punctuation_heavy
}

fn replace_display_urls(text: &str) -> String {
    let text = replace_markdown_display_urls(text);
    let mut output = String::new();
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_whitespace() {
            output.push_str(&display_url_token(&token));
            token.clear();
            output.push(ch);
        } else {
            token.push(ch);
        }
    }
    output.push_str(&display_url_token(&token));
    output
}

fn replace_markdown_display_urls(text: &str) -> String {
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        output.push_str(&rest[..start]);
        let after_open = &rest[start + 1..];
        let Some(label_end) = after_open.find("](") else {
            output.push_str(&rest[start..]);
            return output;
        };
        let url_start = start + 1 + label_end + 2;
        let url_and_after = &rest[url_start..];
        if !(url_and_after.starts_with("http://") || url_and_after.starts_with("https://")) {
            output.push_str(&rest[start..url_start]);
            rest = url_and_after;
            continue;
        }
        let Some(url_end) = url_and_after.find(')') else {
            output.push_str("[link]");
            return output;
        };
        output.push_str("[link]");
        rest = &url_and_after[url_end + 1..];
    }
    output.push_str(rest);
    output
}

fn display_url_token(token: &str) -> String {
    let trimmed = token
        .trim_matches(|ch: char| ch.is_ascii_punctuation() && ch != ':' && ch != '/' && ch != '.');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        "[link]".to_string()
    } else {
        token.to_string()
    }
}

fn split_flattened_list_line(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if colon_heading(trimmed).is_some() {
        return vec![trimmed.to_string()];
    }
    let segments = split_at_inline_markers(trimmed);
    if segments.len() <= 1 {
        vec![line.to_string()]
    } else {
        segments
    }
}

fn split_at_inline_markers(line: &str) -> Vec<String> {
    let mut starts = Vec::new();
    // Walk char boundaries directly: indexing `line` at any `idx` produced by
    // `char_indices` is always valid, which a manual byte cursor is not (a
    // multi-byte char such as an em dash would otherwise be sliced mid-byte).
    for (idx, _) in line.char_indices() {
        let rest = &line[idx..];
        let at_boundary = idx == 0
            || line[..idx]
                .chars()
                .next_back()
                .is_some_and(|ch| ch.is_whitespace() || matches!(ch, ':' | '|' | '.' | '!' | '?'));
        if at_boundary && (starts_numbered_marker(rest) || starts_bullet_marker(rest)) {
            starts.push(idx);
        }
    }

    if starts.len() <= 1 {
        return vec![line.to_string()];
    }

    let mut segments = Vec::new();
    let prefix = line[..starts[0]].trim();
    if !prefix.is_empty() {
        segments.push(prefix.to_string());
    }
    for (idx, start) in starts.iter().enumerate() {
        let end = starts.get(idx + 1).copied().unwrap_or(line.len());
        let segment = line[*start..end].trim();
        if !segment.is_empty() {
            segments.push(segment.to_string());
        }
    }
    segments
}

fn push_blank_once(lines: &mut Vec<String>) {
    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }
}

fn collapse_blanks_between_list_items(lines: Vec<String>) -> Vec<String> {
    let mut collapsed = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.is_empty()
            && idx > 0
            && idx + 1 < lines.len()
            && line_starts_list_item(&lines[idx - 1])
            && line_starts_list_item(&lines[idx + 1])
        {
            continue;
        }
        collapsed.push(line.clone());
    }
    collapsed
}

fn line_starts_list_item(line: &str) -> bool {
    numbered_parts(line).is_some() || bullet_body(line).is_some()
}

pub fn truncate_to_screen(text: String, cols: usize, rows: usize) -> String {
    let max_cols = cols.max(1);
    let max_rows = rows.max(1);
    let mut lines = text
        .lines()
        .take(max_rows)
        .map(|line| {
            let clipped = line.chars().take(max_cols).collect::<String>();
            format!("{clipped:<max_cols$}\r\n")
        })
        .collect::<Vec<_>>();
    while lines.len() < max_rows {
        lines.push(format!("{:<max_cols$}\r\n", ""));
    }
    lines.into_iter().collect()
}

pub(crate) fn place_vn_overlay_text(
    screen: &mut [String],
    cols: usize,
    row: usize,
    col: usize,
    width: usize,
    text: &str,
) {
    let Some(line) = screen.get_mut(row) else {
        return;
    };
    let col = col.min(cols.saturating_sub(1));
    let width = width.min(cols.saturating_sub(col)).max(1);
    // Splice the field into the existing line so multiple fields can share a
    // row as long as their column spans do not overlap.
    let mut chars: Vec<char> = line.chars().collect();
    if chars.len() < cols {
        chars.resize(cols, ' ');
    }
    let field: Vec<char> = format!("{:<width$}", clip_text(text, width))
        .chars()
        .take(width)
        .collect();
    for (i, ch) in field.into_iter().enumerate() {
        if col + i < chars.len() {
            chars[col + i] = ch;
        }
    }
    chars.truncate(cols);
    *line = chars.into_iter().collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose_state::VisualComposeMessage;

    fn assistant_message(text: &str) -> VisualComposeMessage {
        VisualComposeMessage {
            turn_id: 1,
            block_index: 0,
            role: VisualComposeRole::Assistant,
            text: text.to_string(),
            speaker: Some("Codex".to_string()),
            visibility: crate::compose_state::VisualComposeVisibility::Done,
            revealed_chars: None,
        }
    }

    #[test]
    fn inline_marker_split_handles_multibyte_chars_without_panicking() {
        // Regression: an em dash (3 UTF-8 bytes) once caused a mid-char slice
        // panic that killed the Scene overlay thread.
        let line = "Click registered. Nothing new is linked to that action yet—tell me what you want to click next.";
        let segments = split_at_inline_markers(line);
        assert_eq!(segments, vec![line.to_string()]);

        // Real multibyte list content must still split on its markers.
        let listed = split_at_inline_markers("Options— 1. café au lait 2. thé vert");
        assert!(listed.len() >= 2);

        // Whole-reply path must not panic on em-dash prose either.
        let blocks = dialogue_text_blocks(line);
        assert!(!blocks.is_empty());
    }

    #[test]
    fn colon_headings_get_breathing_room() {
        let lines = wrap_compose_transcript_for_vn(
            &[assistant_message("Intro line.\n\nList:\n- first\n- second")],
            80,
        );

        assert_eq!(
            lines,
            vec!["Intro line.", "", "List:", "", "- first", "- second"]
        );
    }

    #[test]
    fn colon_heading_ignores_technical_labels() {
        let lines = wrap_compose_transcript_for_vn(
            &[assistant_message(
                "Status: Ready\nError: command failed\nSee [the report](https://example.com/path).",
            )],
            80,
        );

        assert_eq!(
            lines,
            vec!["Status: Ready", "Error: command failed", "See [link]."]
        );
    }

    #[test]
    fn flattened_numbered_and_bullet_lists_are_split() {
        let lines = wrap_compose_transcript_for_vn(
            &[assistant_message(
                "Next steps: 1. Run the smoke. 2) Check the voice. - Record notes. • Push commits.",
            )],
            80,
        );

        assert_eq!(
            lines,
            vec![
                "Next steps:",
                "",
                "1. Run the smoke.",
                "2. Check the voice.",
                "- Record notes.",
                "- Push commits."
            ]
        );
    }

    #[test]
    fn dialogue_text_blocks_classify_headings_lists_and_technical_lines() {
        let blocks = dialogue_text_blocks(
            "Plan:\n1. Run the smoke.\n- Record notes.\n/Users/julianabeleda/env/gameterm",
        )
        .into_iter()
        .filter(|block| block.kind != VisualDialogueTextBlockKind::Blank)
        .collect::<Vec<_>>();

        assert_eq!(blocks[0].kind, VisualDialogueTextBlockKind::Heading);
        assert_eq!(blocks[0].display_text, "Plan:");
        assert_eq!(blocks[1].kind, VisualDialogueTextBlockKind::Numbered);
        assert_eq!(blocks[1].marker.as_deref(), Some("1"));
        assert_eq!(blocks[1].display_text, "Run the smoke.");
        assert_eq!(blocks[2].kind, VisualDialogueTextBlockKind::Bullet);
        assert_eq!(blocks[2].display_text, "Record notes.");
        assert_eq!(
            blocks[3].kind,
            VisualDialogueTextBlockKind::TechnicalSkipped
        );
        assert!(!blocks[3].speech_allowed());
    }
}
