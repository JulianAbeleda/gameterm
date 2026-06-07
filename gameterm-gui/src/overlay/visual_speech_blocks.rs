const TTS_MAX_SEGMENT_CHARS: usize = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeakableSegment {
    pub(super) turn_id: u64,
    pub(super) block_index: usize,
    pub(super) speaker: Option<String>,
    pub(super) display_text: String,
    pub(super) text: String,
    pub(super) kind: SpeechBlockKind,
    pub(super) source: SpeakableSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpeechBlockKind {
    Prose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpeakableSource {
    ComposeReply,
}

impl SpeakableSource {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            SpeakableSource::ComposeReply => "compose_reply",
        }
    }
}

pub(super) fn extract_speakable_segments(
    speaker: Option<&str>,
    text: &str,
    source: SpeakableSource,
) -> Vec<SpeakableSegment> {
    let speaker = speaker
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let text = strip_fenced_code(text);
    let mut segments = Vec::new();
    let mut current_display = Vec::new();
    let mut current_speakable = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            push_segment(
                &mut segments,
                &speaker,
                &mut current_display,
                &mut current_speakable,
                source,
            );
            continue;
        }
        if is_machine_oriented_line(trimmed) {
            push_segment(
                &mut segments,
                &speaker,
                &mut current_display,
                &mut current_speakable,
                source,
            );
            continue;
        }
        let speakable = clean_speakable_line(trimmed);
        if speakable.is_empty() {
            push_segment(
                &mut segments,
                &speaker,
                &mut current_display,
                &mut current_speakable,
                source,
            );
            continue;
        }
        current_display.push(trimmed.to_string());
        current_speakable.push(speakable);
    }

    push_segment(
        &mut segments,
        &speaker,
        &mut current_display,
        &mut current_speakable,
        source,
    );
    segments
}

fn push_segment(
    segments: &mut Vec<SpeakableSegment>,
    speaker: &Option<String>,
    current_display: &mut Vec<String>,
    current_speakable: &mut Vec<String>,
    source: SpeakableSource,
) {
    if current_speakable.is_empty() {
        return;
    }
    let display_text = current_display.join(" ");
    let text = current_speakable.join(" ");
    current_display.clear();
    current_speakable.clear();
    for text in split_speakable_chunks(&text, TTS_MAX_SEGMENT_CHARS) {
        if text.trim().is_empty() {
            continue;
        }
        segments.push(SpeakableSegment {
            turn_id: 0,
            block_index: segments.len(),
            speaker: speaker.clone(),
            display_text: display_text.clone(),
            text,
            kind: SpeechBlockKind::Prose,
            source,
        });
    }
}

fn strip_fenced_code(text: &str) -> String {
    let mut output = String::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn is_machine_oriented_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if line.starts_with("diff --git")
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
    {
        return true;
    }

    let path_like = (line.matches('/').count() >= 2 || line.matches('\\').count() >= 2)
        && line.split_whitespace().count() <= 4;
    let identifier_chars = line
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '\\' | '.' | ':'))
        .count();
    let total_chars = line.chars().count().max(1);
    let identifier_heavy = identifier_chars * 100 / total_chars > 85
        && line.split_whitespace().count() <= 4
        && !line.contains(' ');
    let punctuation_heavy =
        line.chars().filter(|ch| ch.is_ascii_punctuation()).count() * 100 / total_chars > 45;

    path_like || identifier_heavy || punctuation_heavy
}

fn clean_speakable_line(line: &str) -> String {
    let without_inline_code = replace_inline_code(line);
    let without_urls = replace_url_spans(&without_inline_code);
    let mut cleaned = Vec::new();
    let mut last_replacement: Option<&'static str> = None;
    for word in without_urls.split_whitespace() {
        let replacement = speakable_word(word);
        match replacement {
            WordSpeech::Keep(value) => {
                cleaned.push(value);
                last_replacement = None;
            }
            WordSpeech::Replace(value) => {
                if last_replacement != Some(value) {
                    cleaned.push(value.to_string());
                }
                last_replacement = Some(value);
            }
            WordSpeech::Skip => {}
        }
    }
    cleaned.join(" ").trim().to_string()
}

fn replace_inline_code(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('`') else {
            output.push_str(after_start);
            return output;
        };
        let code = &after_start[..end];
        if inline_code_is_technical(code) {
            output.push_str(" the command ");
        } else {
            output.push_str(code);
        }
        rest = &after_start[end + 1..];
    }
    output.push_str(rest);
    output
}

fn replace_url_spans(line: &str) -> String {
    let line = replace_markdown_url_spans(line);
    let mut output = String::new();
    let mut token = String::new();
    for ch in line.chars() {
        if ch.is_whitespace() {
            output.push_str(&speakable_url_token(&token));
            token.clear();
            output.push(ch);
        } else {
            token.push(ch);
        }
    }
    output.push_str(&speakable_url_token(&token));
    output
}

fn replace_markdown_url_spans(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;
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
            output.push_str(" the link ");
            return output;
        };
        output.push_str(" the link ");
        rest = &url_and_after[url_end + 1..];
    }
    output.push_str(rest);
    output
}

fn speakable_url_token(token: &str) -> String {
    if token.contains("http://") || token.contains("https://") {
        " the link ".to_string()
    } else {
        token.to_string()
    }
}

fn inline_code_is_technical(code: &str) -> bool {
    let trimmed = code.trim();
    trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('=')
        || trimmed.starts_with("--")
        || trimmed.split_whitespace().count() > 1
        || matches!(
            trimmed.split_whitespace().next(),
            Some("cargo" | "git" | "make" | "npm" | "pnpm" | "python" | "rustc")
        )
}

enum WordSpeech {
    Keep(String),
    Replace(&'static str),
    Skip,
}

fn speakable_word(word: &str) -> WordSpeech {
    if word.chars().all(|ch| ch.is_ascii_punctuation()) {
        return WordSpeech::Skip;
    }
    let raw = word.trim_matches(|ch: char| ch.is_ascii_punctuation() && ch != '-');
    if is_flag_token(raw) {
        return WordSpeech::Skip;
    }
    let trimmed = word.trim_matches(|ch: char| {
        ch.is_ascii_punctuation() && ch != '/' && ch != '\\' && ch != ':' && ch != '.'
    });
    if trimmed.is_empty() {
        return WordSpeech::Skip;
    }
    if is_url_token(trimmed) {
        return WordSpeech::Replace("the link");
    }
    if is_unix_path_token(trimmed) {
        return if path_token_looks_like_file(trimmed) {
            WordSpeech::Replace("that file")
        } else {
            WordSpeech::Replace("the project folder")
        };
    }
    if is_windows_path_token(trimmed) {
        return WordSpeech::Replace("that folder");
    }
    if is_commit_hash_token(trimmed) || is_env_var_token(trimmed) {
        return WordSpeech::Skip;
    }
    if file_name_token_looks_technical(trimmed) {
        return WordSpeech::Replace("that file");
    }
    WordSpeech::Keep(word.replace(['`', '*'], ""))
}

fn is_url_token(token: &str) -> bool {
    token.starts_with("http://") || token.starts_with("https://")
}

fn is_unix_path_token(token: &str) -> bool {
    token.starts_with('/') && token.matches('/').count() >= 2
}

fn is_windows_path_token(token: &str) -> bool {
    token.len() > 3
        && token.as_bytes().get(1) == Some(&b':')
        && token.as_bytes().get(2) == Some(&b'\\')
}

fn path_token_looks_like_file(token: &str) -> bool {
    token
        .rsplit(['/', '\\'])
        .next()
        .is_some_and(file_name_token_looks_technical)
}

fn is_commit_hash_token(token: &str) -> bool {
    (7..=40).contains(&token.len()) && token.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_env_var_token(token: &str) -> bool {
    token.len() >= 4
        && token.contains('_')
        && token
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_flag_token(token: &str) -> bool {
    token.starts_with("--") && token.len() > 2
}

fn file_name_token_looks_technical(token: &str) -> bool {
    const EXTENSIONS: [&str; 18] = [
        ".rs", ".toml", ".json", ".yaml", ".yml", ".md", ".sh", ".py", ".js", ".ts", ".tsx",
        ".jsx", ".lock", ".png", ".jpg", ".jpeg", ".wav", ".zip",
    ];
    let lower = token
        .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':' | '?' | '!'))
        .to_ascii_lowercase();
    EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

fn split_speakable_chunks(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let mut chunks = Vec::new();
    let mut current = String::new();
    for sentence in split_sentences(text) {
        push_chunk_part(&mut chunks, &mut current, sentence.trim(), max_chars);
    }
    push_current_chunk(&mut chunks, &mut current);
    chunks
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        current.push(ch);
        let ends_sentence = matches!(ch, '.' | '!' | '?')
            && match chars.peek() {
                Some(next) => next.is_whitespace() || matches!(next, '"' | '\''),
                None => true,
            };
        if ends_sentence {
            sentences.push(current.trim().to_string());
            current.clear();
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences
}

fn push_chunk_part(chunks: &mut Vec<String>, current: &mut String, part: &str, max_chars: usize) {
    if part.is_empty() {
        return;
    }
    if part.chars().count() > max_chars {
        push_current_chunk(chunks, current);
        for split in split_long_part_by_words(part, max_chars) {
            push_chunk_part(chunks, current, &split, max_chars);
        }
        return;
    }
    let separator = usize::from(!current.is_empty());
    if current.chars().count() + separator + part.chars().count() > max_chars {
        push_current_chunk(chunks, current);
    }
    if !current.is_empty() {
        current.push(' ');
    }
    current.push_str(part);
}

fn split_long_part_by_words(part: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for word in part.split_whitespace() {
        if word.chars().count() > max_chars {
            push_current_chunk(&mut chunks, &mut current);
            chunks.extend(split_long_word(word, max_chars));
            continue;
        }
        let separator = usize::from(!current.is_empty());
        if current.chars().count() + separator + word.chars().count() > max_chars {
            push_current_chunk(&mut chunks, &mut current);
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    push_current_chunk(&mut chunks, &mut current);
    chunks
}

fn split_long_word(word: &str, max_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in word.chars() {
        if current.chars().count() >= max_chars {
            chunks.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn push_current_chunk(chunks: &mut Vec<String>, current: &mut String) {
    let chunk = current.trim();
    if !chunk.is_empty() {
        chunks.push(chunk.to_string());
    }
    current.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_speech_blocks_extract_prose_and_skip_code_and_logs() {
        let text = r#"Here is the plan.

```rust
fn main() {}
```

diff --git a/file b/file
error: command failed
We can continue after the smoke pass."#;

        let segments =
            extract_speakable_segments(Some("Codex"), text, SpeakableSource::ComposeReply);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Here is the plan.");
        assert_eq!(segments[1].text, "We can continue after the smoke pass.");
        assert_eq!(segments[0].speaker.as_deref(), Some("Codex"));
        assert_eq!(segments[0].block_index, 0);
        assert_eq!(segments[1].block_index, 1);
        assert_eq!(segments[0].kind, SpeechBlockKind::Prose);
    }

    #[test]
    fn visual_speech_blocks_clean_inline_technical_spans_without_changing_display_text() {
        let text = "I updated /Users/julianabeleda/env/gameterm and ran `cargo test -p gameterm-gui`. See [the report](https://example.com/report).";

        let segments =
            extract_speakable_segments(Some("Codex"), text, SpeakableSource::ComposeReply);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].display_text, text);
        assert_eq!(
            segments[0].text,
            "I updated the project folder and ran the command See the link"
        );
    }

    #[test]
    fn visual_speech_blocks_split_long_segments_without_dropping_text() {
        let sentence = "This sentence keeps enough natural words to exceed the speech chunk limit when repeated.";
        let text = (0..24).map(|_| sentence).collect::<Vec<_>>().join(" ");

        let segments =
            extract_speakable_segments(Some("Codex"), &text, SpeakableSource::ComposeReply);

        assert!(segments.len() > 1);
        assert!(segments
            .iter()
            .all(|segment| segment.text.chars().count() <= TTS_MAX_SEGMENT_CHARS));
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            text
        );
        assert_eq!(segments[0].block_index, 0);
        assert_eq!(segments[1].block_index, 1);
    }

    #[test]
    fn visual_speech_blocks_skip_hashes_flags_env_vars_and_replace_files() {
        let text =
            "Commit 1234abcd updated GAMETERM_SCENE_TTS_BACKEND with --verbose in Cargo.toml.";

        let segments =
            extract_speakable_segments(Some("Codex"), text, SpeakableSource::ComposeReply);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Commit updated with in that file");
    }
}
