use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const TTS_BACKEND_ENV: &str = "GAMETERM_SCENE_TTS_BACKEND";
const TTS_COMMAND_ENV: &str = "GAMETERM_SCENE_TTS_COMMAND";
const TTS_PLAYER_ENV: &str = "GAMETERM_SCENE_TTS_PLAYER";
const TTS_CACHE_DIR_ENV: &str = "GAMETERM_SCENE_TTS_CACHE_DIR";
const TTS_TIMEOUT_ENV: &str = "GAMETERM_SCENE_TTS_TIMEOUT_SECONDS";
const TTS_DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const TTS_MAX_SEGMENT_CHARS: usize = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeakableSegment {
    pub(super) speaker: Option<String>,
    pub(super) text: String,
    pub(super) source: SpeakableSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpeakableSource {
    ComposeReply,
}

impl SpeakableSource {
    fn as_str(self) -> &'static str {
        match self {
            SpeakableSource::ComposeReply => "compose_reply",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsRequest {
    pub(super) segment: SpeakableSegment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsResult {
    pub(super) status: String,
    pub(super) output_path: Option<PathBuf>,
    pub(super) error: Option<String>,
}

impl SceneTtsResult {
    pub(super) fn succeeded(&self) -> bool {
        self.error.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SceneTtsBackend {
    Disabled,
    BuiltInSilent,
    Command(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneTtsConfig {
    backend: SceneTtsBackend,
    player: Option<Vec<String>>,
    cache_dir: PathBuf,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsState {
    muted: bool,
    last_status: String,
}

impl Default for SceneTtsState {
    fn default() -> Self {
        Self {
            muted: false,
            last_status: "TTS idle".to_string(),
        }
    }
}

impl SceneTtsState {
    pub(super) fn toggle_muted(&mut self) -> String {
        self.muted = !self.muted;
        self.last_status = if self.muted {
            "TTS muted".to_string()
        } else {
            "TTS unmuted".to_string()
        };
        self.last_status.clone()
    }

    pub(super) fn is_muted(&self) -> bool {
        self.muted
    }

    pub(super) fn apply_result(&mut self, result: &SceneTtsResult) -> String {
        self.last_status = result.status.clone();
        self.last_status.clone()
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
    let mut current = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            push_segment(&mut segments, &speaker, &mut current, source);
            continue;
        }
        if is_machine_oriented_line(trimmed) {
            push_segment(&mut segments, &speaker, &mut current, source);
            continue;
        }
        current.push(trimmed.to_string());
    }

    push_segment(&mut segments, &speaker, &mut current, source);
    segments
}

pub(super) fn spawn_tts_backend(request: SceneTtsRequest, tx: mpsc::Sender<SceneTtsResult>) {
    thread::spawn(move || {
        let result = run_tts_backend(request, scene_tts_config_from_env());
        let _ = tx.send(result);
    });
}

fn push_segment(
    segments: &mut Vec<SpeakableSegment>,
    speaker: &Option<String>,
    current: &mut Vec<String>,
    source: SpeakableSource,
) {
    if current.is_empty() {
        return;
    }
    let text = current.join(" ");
    current.clear();
    let text = text.chars().take(TTS_MAX_SEGMENT_CHARS).collect::<String>();
    if text.trim().is_empty() {
        return;
    }
    segments.push(SpeakableSegment {
        speaker: speaker.clone(),
        text,
        source,
    });
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

    let path_like = line.matches('/').count() >= 2 || line.matches('\\').count() >= 2;
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

fn scene_tts_config_from_env() -> SceneTtsConfig {
    let backend = match std::env::var(TTS_BACKEND_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("silent") | Some("test") | Some("builtin") => SceneTtsBackend::BuiltInSilent,
        Some("command") => match std::env::var(TTS_COMMAND_ENV)
            .ok()
            .and_then(|value| parse_command_argv(&value).ok())
            .filter(|argv| !argv.is_empty())
        {
            Some(argv) => SceneTtsBackend::Command(argv),
            None => SceneTtsBackend::Disabled,
        },
        _ => SceneTtsBackend::Disabled,
    };

    SceneTtsConfig {
        backend,
        player: std::env::var(TTS_PLAYER_ENV)
            .ok()
            .and_then(|value| parse_command_argv(&value).ok())
            .filter(|argv| !argv.is_empty()),
        cache_dir: std::env::var(TTS_CACHE_DIR_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir),
        timeout: std::env::var(TTS_TIMEOUT_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(TTS_DEFAULT_TIMEOUT),
    }
}

fn run_tts_backend(request: SceneTtsRequest, config: SceneTtsConfig) -> SceneTtsResult {
    match &config.backend {
        SceneTtsBackend::Disabled => SceneTtsResult {
            status: "TTS disabled".to_string(),
            output_path: None,
            error: None,
        },
        SceneTtsBackend::BuiltInSilent => SceneTtsResult {
            status: format!("TTS silent: {}", request.segment.text),
            output_path: None,
            error: None,
        },
        SceneTtsBackend::Command(argv) => run_command_tts_backend(request, &config, argv.clone()),
    }
}

fn run_command_tts_backend(
    request: SceneTtsRequest,
    config: &SceneTtsConfig,
    argv: Vec<String>,
) -> SceneTtsResult {
    let output_path = tts_output_path(&config.cache_dir);
    let argv = argv
        .into_iter()
        .map(|arg| arg.replace("{output}", &output_path.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = argv.split_first() else {
        return SceneTtsResult {
            status: "TTS failed".to_string(),
            output_path: None,
            error: Some("empty TTS command".to_string()),
        };
    };

    let mut child = match Command::new(program)
        .args(args)
        .env("GAMETERM_SCENE_TTS_OUTPUT", &output_path)
        .env(
            "GAMETERM_SCENE_TTS_SPEAKER",
            request.segment.speaker.as_deref().unwrap_or(""),
        )
        .env("GAMETERM_SCENE_TTS_SOURCE", request.segment.source.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return SceneTtsResult {
                status: "TTS failed".to_string(),
                output_path: None,
                error: Some(format!("failed to spawn TTS command `{program}`: {err}")),
            };
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(request.segment.text.as_bytes());
    }
    drop(child.stdin.take());

    let output = match wait_for_tts_output(child, config.timeout) {
        Ok(output) => output,
        Err(err) => {
            return SceneTtsResult {
                status: "TTS failed".to_string(),
                output_path: None,
                error: Some(err),
            };
        }
    };

    if !output.status.success() {
        return SceneTtsResult {
            status: "TTS failed".to_string(),
            output_path: None,
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        };
    }

    if let Some(player) = &config.player {
        let _ = run_player_command(player.clone(), &output_path);
    }

    SceneTtsResult {
        status: format!("TTS ready: {}", output_path.display()),
        output_path: Some(output_path),
        error: None,
    }
}

fn wait_for_tts_output(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(|err| err.to_string()),
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "TTS command timed out after {}s",
                    timeout.as_secs()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(err) => return Err(err.to_string()),
        }
    }
}

fn run_player_command(argv: Vec<String>, output_path: &PathBuf) -> Result<(), String> {
    let argv = argv
        .into_iter()
        .map(|arg| arg.replace("{output}", &output_path.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = argv.split_first() else {
        return Ok(());
    };
    Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|err| err.to_string())
}

fn tts_output_path(cache_dir: &PathBuf) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    cache_dir.join(format!(
        "gameterm-scene-tts-{}-{stamp}.wav",
        std::process::id()
    ))
}

fn parse_command_argv(command: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' | '"' => match quote {
                Some(active) if active == ch => quote = None,
                None => quote = Some(ch),
                _ => current.push(ch),
            },
            ch if ch.is_whitespace() && quote.is_none() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if let Some(active) = quote {
        return Err(format!("unterminated {active} quote"));
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_tts_extracts_prose_and_skips_code_and_logs() {
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
    }

    #[test]
    fn visual_tts_parses_command_argv_with_quotes() {
        assert_eq!(
            parse_command_argv(r#"tts-helper --voice "Agent Voice" --output {output}"#).unwrap(),
            vec![
                "tts-helper",
                "--voice",
                "Agent Voice",
                "--output",
                "{output}"
            ]
        );
    }

    #[test]
    fn visual_tts_command_backend_writes_output() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("tts-helper.sh");
        std::fs::write(
            &helper,
            "#!/usr/bin/env sh\ncat > \"$GAMETERM_SCENE_TTS_OUTPUT\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&helper, permissions).unwrap();
        }

        let result = run_tts_backend(
            SceneTtsRequest {
                segment: SpeakableSegment {
                    speaker: Some("Codex".to_string()),
                    text: "Speak this line.".to_string(),
                    source: SpeakableSource::ComposeReply,
                },
            },
            SceneTtsConfig {
                backend: SceneTtsBackend::Command(vec![helper.display().to_string()]),
                player: None,
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
        );

        assert!(result.succeeded());
        let output_path = result.output_path.unwrap();
        assert_eq!(
            std::fs::read_to_string(output_path).unwrap(),
            "Speak this line."
        );
    }

    #[test]
    fn visual_tts_state_toggles_mute() {
        let mut state = SceneTtsState::default();
        assert!(!state.is_muted());
        assert_eq!(state.toggle_muted(), "TTS muted");
        assert!(state.is_muted());
    }
}
