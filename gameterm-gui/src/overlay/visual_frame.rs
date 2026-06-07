pub(super) fn replace_last_screen_line(
    frame: String,
    cols: usize,
    rows: usize,
    replacement: &str,
) -> String {
    let rows = rows.max(1);
    replace_screen_line(frame, cols, rows, rows - 1, replacement)
}

pub(super) fn replace_screen_line(
    frame: String,
    cols: usize,
    rows: usize,
    target_row: usize,
    replacement: &str,
) -> String {
    let rows = rows.max(1);
    let cols = cols.max(1);
    let mut lines = frame.lines().map(str::to_string).collect::<Vec<_>>();
    while lines.len() < rows {
        lines.push(String::new());
    }
    lines.truncate(rows);
    lines[target_row.min(rows - 1)] = fixed_width_text(replacement, cols);
    let mut out = lines
        .into_iter()
        .map(|line| fixed_width_text(&line, cols))
        .collect::<Vec<_>>()
        .join("\r\n");
    out.push_str("\r\n");
    out
}

pub(super) fn clip_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn fixed_width_text(text: &str, cols: usize) -> String {
    let clipped = clip_text(text, cols);
    format!("{clipped:<cols$}")
}
