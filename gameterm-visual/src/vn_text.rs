use crate::runtime_status::{clip_text, wrap_text};
use crate::{VisualComposeMessage, VisualComposeRole};

pub(crate) fn wrap_compose_transcript_for_vn(
    messages: &[VisualComposeMessage],
    dialogue_width: usize,
) -> Vec<String> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for message in messages {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(wrap_compose_message_for_vn(message, dialogue_width));
    }
    lines
}

fn wrap_compose_message_for_vn(
    message: &VisualComposeMessage,
    dialogue_width: usize,
) -> Vec<String> {
    match message.role {
        VisualComposeRole::User => wrap_user_prompt_for_vn(&message.text, dialogue_width),
        VisualComposeRole::Assistant | VisualComposeRole::System => {
            wrap_dialogue_display_text(&message.text, dialogue_width)
        }
        VisualComposeRole::Error => wrap_error_for_vn(&message.text, dialogue_width),
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
    let display_text = structured_dialogue_text(text).unwrap_or_else(|| text.trim().to_string());
    let normalized = normalize_dialogue_blocks(&display_text);
    let mut lines = Vec::new();
    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            push_blank_once(&mut lines);
            continue;
        }
        if let Some((marker, body)) = numbered_parts(line) {
            lines.extend(wrap_marked_line(marker, body, dialogue_width));
            continue;
        }
        if let Some(body) = bullet_body(line) {
            lines.extend(wrap_marked_line("-", body, dialogue_width));
            continue;
        }
        lines.extend(wrap_text(&strip_inline_markers(line), dialogue_width));
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
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
    let text = insert_section_breaks(&text);
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
            output.push(line.to_string());
        }
    }
    output.join("\n")
}

fn insert_section_breaks(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if idx > 0 && starts_numbered_marker(&text[idx..]) && should_break_before(text, idx) {
            output.push('\n');
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

fn starts_numbered_marker(text: &str) -> bool {
    let text = text
        .strip_prefix("**")
        .or_else(|| text.strip_prefix("__"))
        .unwrap_or(text);
    numbered_parts(text).is_some()
}

fn numbered_parts(line: &str) -> Option<(&str, &str)> {
    let (marker, body) = line.trim().split_once(". ")?;
    if marker.chars().all(|ch| ch.is_ascii_digit()) {
        Some((marker, body))
    } else if let Some(marker) = marker.strip_prefix("**") {
        marker
            .chars()
            .all(|ch| ch.is_ascii_digit())
            .then_some((marker, body))
    } else {
        None
    }
}

fn bullet_body(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
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

fn push_blank_once(lines: &mut Vec<String>) {
    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }
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
