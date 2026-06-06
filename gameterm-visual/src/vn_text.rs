use crate::runtime_status::{clip_text, wrap_text};

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

pub fn truncate_to_screen(text: String, cols: usize, rows: usize) -> String {
    let max_cols = cols.max(1);
    text.lines()
        .take(rows.max(1))
        .map(|line| {
            let mut clipped = line.chars().take(max_cols).collect::<String>();
            clipped.push_str("\r\n");
            clipped
        })
        .collect()
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
