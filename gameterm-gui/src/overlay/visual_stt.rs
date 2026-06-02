use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const STT_BACKEND_ENV: &str = "GAMETERM_SCENE_STT_BACKEND";
const STT_COMMAND_ENV: &str = "GAMETERM_SCENE_STT_COMMAND";
const STT_TIMEOUT_ENV: &str = "GAMETERM_SCENE_STT_TIMEOUT_SECONDS";
const STT_AUTO_SUBMIT_ENV: &str = "GAMETERM_SCENE_STT_AUTO_SUBMIT";
const STT_DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const STT_MAX_TRANSCRIPT_CHARS: usize = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneSttResult {
    pub(super) status: String,
    pub(super) transcript: Option<String>,
    pub(super) auto_submit: bool,
    pub(super) error: Option<String>,
}

impl SceneSttResult {
    pub(super) fn succeeded(&self) -> bool {
        self.error.is_none() && self.transcript.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SceneSttBackend {
    Disabled,
    Command(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneSttConfig {
    backend: SceneSttBackend,
    timeout: Duration,
    auto_submit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneSttState {
    running: bool,
    last_status: String,
}

impl Default for SceneSttState {
    fn default() -> Self {
        Self {
            running: false,
            last_status: "Voice idle".to_string(),
        }
    }
}

impl SceneSttState {
    pub(super) fn is_running(&self) -> bool {
        self.running
    }

    pub(super) fn mark_started(&mut self) -> String {
        self.running = true;
        self.last_status = "Voice listening".to_string();
        self.last_status.clone()
    }

    pub(super) fn mark_canceling(&mut self) -> String {
        self.last_status = "Voice canceling".to_string();
        self.last_status.clone()
    }

    pub(super) fn apply_result(&mut self, result: &SceneSttResult) -> String {
        self.running = false;
        self.last_status = result.status.clone();
        self.last_status.clone()
    }
}

#[derive(Debug)]
pub(super) struct SceneSttCancel {
    tx: mpsc::Sender<()>,
}

impl SceneSttCancel {
    pub(super) fn cancel(&self) {
        let _ = self.tx.send(());
    }
}

pub(super) fn spawn_stt_backend(tx: mpsc::Sender<SceneSttResult>) -> SceneSttCancel {
    let config = scene_stt_config_from_env();
    let (cancel_tx, cancel_rx) = mpsc::channel();
    thread::spawn(move || {
        let result = run_stt_backend(config, cancel_rx);
        let _ = tx.send(result);
    });
    SceneSttCancel { tx: cancel_tx }
}

fn scene_stt_config_from_env() -> SceneSttConfig {
    let backend = match std::env::var(STT_BACKEND_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("command") => match std::env::var(STT_COMMAND_ENV)
            .ok()
            .and_then(|value| parse_command_argv(&value).ok())
            .filter(|argv| !argv.is_empty())
        {
            Some(argv) => SceneSttBackend::Command(argv),
            None => SceneSttBackend::Disabled,
        },
        _ => SceneSttBackend::Disabled,
    };

    SceneSttConfig {
        backend,
        timeout: std::env::var(STT_TIMEOUT_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(STT_DEFAULT_TIMEOUT),
        auto_submit: std::env::var(STT_AUTO_SUBMIT_ENV)
            .ok()
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false),
    }
}

fn run_stt_backend(config: SceneSttConfig, cancel_rx: mpsc::Receiver<()>) -> SceneSttResult {
    match &config.backend {
        SceneSttBackend::Disabled => SceneSttResult {
            status: "Voice disabled".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some("STT backend is disabled".to_string()),
        },
        SceneSttBackend::Command(argv) => run_command_stt_backend(&config, argv.clone(), cancel_rx),
    }
}

fn run_command_stt_backend(
    config: &SceneSttConfig,
    argv: Vec<String>,
    cancel_rx: mpsc::Receiver<()>,
) -> SceneSttResult {
    let Some((program, args)) = argv.split_first() else {
        return SceneSttResult {
            status: "Voice failed".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some("empty STT command".to_string()),
        };
    };

    let mut child = match Command::new(program)
        .args(args)
        .env("GAMETERM_SCENE_STT_MODE", "scene")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return SceneSttResult {
                status: "Voice failed".to_string(),
                transcript: None,
                auto_submit: false,
                error: Some(format!("failed to spawn STT command `{program}`: {err}")),
            };
        }
    };

    let started = std::time::Instant::now();
    loop {
        if cancel_rx.try_recv().is_ok() {
            let _ = child.kill();
            return SceneSttResult {
                status: "Voice canceled".to_string(),
                transcript: None,
                auto_submit: false,
                error: Some("STT canceled".to_string()),
            };
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= config.timeout => {
                let _ = child.kill();
                return SceneSttResult {
                    status: "Voice failed".to_string(),
                    transcript: None,
                    auto_submit: false,
                    error: Some("STT command timed out".to_string()),
                };
            }
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(err) => {
                let _ = child.kill();
                return SceneSttResult {
                    status: "Voice failed".to_string(),
                    transcript: None,
                    auto_submit: false,
                    error: Some(err.to_string()),
                };
            }
        }
    }

    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }
    let status = child.wait().ok();
    if !status.as_ref().is_some_and(|status| status.success()) {
        return SceneSttResult {
            status: "Voice failed".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some(stderr.trim().to_string()),
        };
    }

    let transcript = sanitize_transcript(&stdout);
    if transcript.is_empty() {
        return SceneSttResult {
            status: "Voice failed".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some("empty transcript".to_string()),
        };
    }

    SceneSttResult {
        status: "Voice transcript ready".to_string(),
        transcript: Some(transcript),
        auto_submit: config.auto_submit,
        error: None,
    }
}

pub(super) fn sanitize_transcript(raw: &str) -> String {
    let mut text = raw.replace('\r', "\n");
    for artifact in ["[BLANK_AUDIO]", "[MUSIC]", "(silence)", "<|nospeech|>"] {
        text = text.replace(artifact, "");
    }
    let mut normalized = String::new();
    let mut last_was_space = false;
    for ch in text
        .chars()
        .filter(|ch| !ch.is_control() || ch.is_whitespace())
    {
        if ch.is_whitespace() {
            if !last_was_space {
                normalized.push(' ');
                last_was_space = true;
            }
        } else {
            normalized.push(ch);
            last_was_space = false;
        }
        if normalized.chars().count() >= STT_MAX_TRANSCRIPT_CHARS {
            normalized.push_str("...");
            break;
        }
    }
    normalized.trim().to_string()
}

fn parse_command_argv(command: &str) -> Result<Vec<String>, String> {
    let mut argv = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match (ch, quote) {
            ('\\', Some(_)) => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            ('"', None) | ('\'', None) => quote = Some(ch),
            ('"', Some('"')) | ('\'', Some('\'')) => quote = None,
            (ch, None) if ch.is_whitespace() => {
                if !current.is_empty() {
                    argv.push(std::mem::take(&mut current));
                }
            }
            (ch, _) => current.push(ch),
        }
    }
    if let Some(ch) = quote {
        return Err(format!("unterminated quote `{ch}` in STT command"));
    }
    if !current.is_empty() {
        argv.push(current);
    }
    Ok(argv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_stt_sanitizes_transcript_artifacts() {
        let transcript = sanitize_transcript("  hello\r\n[MUSIC]\nworld <|nospeech|>  ");
        assert_eq!(transcript, "hello world");
    }

    #[test]
    fn visual_stt_parses_command_argv_with_quotes() {
        assert_eq!(
            parse_command_argv(r#"stt-helper --model "small model""#).unwrap(),
            vec!["stt-helper", "--model", "small model"]
        );
    }

    #[test]
    fn visual_stt_command_backend_returns_transcript() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("stt-helper.sh");
        std::fs::write(&helper, "#!/usr/bin/env sh\nprintf 'open the roadmap\\n'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&helper, permissions).unwrap();
        }

        let (_cancel_tx, cancel_rx) = mpsc::channel();
        let result = run_stt_backend(
            SceneSttConfig {
                backend: SceneSttBackend::Command(vec![helper.display().to_string()]),
                timeout: Duration::from_secs(2),
                auto_submit: false,
            },
            cancel_rx,
        );

        assert!(result.succeeded());
        assert_eq!(result.transcript.as_deref(), Some("open the roadmap"));
    }
}
