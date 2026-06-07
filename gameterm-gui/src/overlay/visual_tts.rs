use super::visual_speech_blocks::SpeakableSegment;
#[cfg(test)]
use super::visual_speech_blocks::{SpeakableSource, SpeechBlockKind};
use std::io::Read;
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
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
const TTS_VOICEVOX_HOST_ENV: &str = "GAMETERM_SCENE_TTS_VOICEVOX_HOST";
const TTS_VOICEVOX_PORT_ENV: &str = "GAMETERM_SCENE_TTS_VOICEVOX_PORT";
const TTS_VOICEVOX_SPEAKER_ENV: &str = "GAMETERM_SCENE_TTS_VOICEVOX_SPEAKER";
const TTS_TRANSLATION_BACKEND_ENV: &str = "GAMETERM_SCENE_TTS_TRANSLATION_BACKEND";
const TTS_TRANSLATION_COMMAND_ENV: &str = "GAMETERM_SCENE_TTS_TRANSLATE_COMMAND";
const TTS_CT2_COMMAND_ENV: &str = "GAMETERM_SCENE_TTS_CT2_COMMAND";
const TTS_DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsRequest {
    pub(super) segment: SpeakableSegment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsResult {
    pub(super) event: SceneTtsEvent,
    pub(super) segment: SpeakableSegment,
    pub(super) status: String,
    pub(super) output_path: Option<PathBuf>,
    pub(super) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SceneTtsEvent {
    Started,
    Finished,
}

impl SceneTtsResult {
    pub(super) fn succeeded(&self) -> bool {
        self.error.is_none()
    }
}

fn finished_tts_result(
    segment: SpeakableSegment,
    status: impl Into<String>,
    output_path: Option<PathBuf>,
    error: Option<String>,
) -> SceneTtsResult {
    SceneTtsResult {
        event: SceneTtsEvent::Finished,
        segment,
        status: status.into(),
        output_path,
        error,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SceneTtsBackend {
    Disabled,
    BuiltInSilent,
    Command(Vec<String>),
    Voicevox(SceneVoicevoxConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneTtsConfig {
    backend: SceneTtsBackend,
    player: Option<Vec<String>>,
    cache_dir: PathBuf,
    timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneVoicevoxConfig {
    host: String,
    port: u16,
    speaker: u32,
    translation: SceneTranslationConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SceneTranslationConfig {
    Off,
    Command(Vec<String>),
}

impl SceneTtsConfig {
    pub(crate) fn from_env() -> Self {
        scene_tts_config_from_env()
    }

    pub(crate) fn voicevox_default() -> Self {
        Self {
            backend: SceneTtsBackend::Voicevox(SceneVoicevoxConfig::from_env()),
            player: Some(vec!["afplay".to_string(), "{output}".to_string()]),
            cache_dir: std::env::temp_dir(),
            timeout: Duration::from_secs(120),
        }
    }

    #[cfg(test)]
    pub(crate) fn can_play_audio(&self) -> bool {
        matches!(
            self.backend,
            SceneTtsBackend::Command(_) | SceneTtsBackend::Voicevox(_)
        )
    }

    #[cfg(test)]
    pub(crate) fn is_voicevox_backend(&self) -> bool {
        matches!(self.backend, SceneTtsBackend::Voicevox(_))
    }
}

impl SceneVoicevoxConfig {
    fn from_env() -> Self {
        let host = env_first_non_empty(&[TTS_VOICEVOX_HOST_ENV, "VOICEVOX_HOST"])
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = env_first_non_empty(&[TTS_VOICEVOX_PORT_ENV, "VOICEVOX_PORT"])
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(50021);
        let speaker = env_first_non_empty(&[TTS_VOICEVOX_SPEAKER_ENV, "VOICEVOX_SPEAKER"])
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(14);
        Self {
            host,
            port,
            speaker,
            translation: SceneTranslationConfig::from_env(),
        }
    }
}

impl SceneTranslationConfig {
    fn from_env() -> Self {
        if let Some(command) = env_first_non_empty(&[TTS_TRANSLATION_COMMAND_ENV]) {
            return parse_command_argv(&command)
                .ok()
                .filter(|argv| !argv.is_empty())
                .map(SceneTranslationConfig::Command)
                .unwrap_or(SceneTranslationConfig::Off);
        }

        match env_first_non_empty(&[TTS_TRANSLATION_BACKEND_ENV])
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("off") | Some("none") => SceneTranslationConfig::Off,
            Some("command") => std::env::var(TTS_TRANSLATION_COMMAND_ENV)
                .ok()
                .and_then(|value| parse_command_argv(&value).ok())
                .filter(|argv| !argv.is_empty())
                .map(SceneTranslationConfig::Command)
                .unwrap_or(SceneTranslationConfig::Off),
            Some("ct2") | None => ct2_translation_command()
                .map(SceneTranslationConfig::Command)
                .unwrap_or(SceneTranslationConfig::Off),
            _ => SceneTranslationConfig::Off,
        }
    }
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

enum SceneTtsWorkerMessage {
    Speak(SceneTtsRequest),
    Shutdown,
}

pub(super) struct SceneTtsWorker {
    tx: mpsc::Sender<SceneTtsWorkerMessage>,
}

impl SceneTtsWorker {
    pub(super) fn new(config: SceneTtsConfig, result_tx: mpsc::Sender<SceneTtsResult>) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                match message {
                    SceneTtsWorkerMessage::Speak(request) => {
                        let _ = result_tx.send(SceneTtsResult {
                            event: SceneTtsEvent::Started,
                            segment: request.segment.clone(),
                            status: "TTS speaking".to_string(),
                            output_path: None,
                            error: None,
                        });
                        let result = run_tts_backend(request, config.clone());
                        let _ = result_tx.send(result);
                    }
                    SceneTtsWorkerMessage::Shutdown => break,
                }
            }
        });
        Self { tx }
    }

    pub(super) fn speak(&self, request: SceneTtsRequest) {
        let _ = self.tx.send(SceneTtsWorkerMessage::Speak(request));
    }
}

impl Drop for SceneTtsWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(SceneTtsWorkerMessage::Shutdown);
    }
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
        Some("voicevox") => SceneTtsBackend::Voicevox(SceneVoicevoxConfig::from_env()),
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
        SceneTtsBackend::Disabled => {
            finished_tts_result(request.segment, "TTS disabled", None, None)
        }
        SceneTtsBackend::BuiltInSilent => finished_tts_result(
            request.segment.clone(),
            format!("TTS silent: {}", request.segment.text),
            None,
            None,
        ),
        SceneTtsBackend::Command(argv) => run_command_tts_backend(request, &config, argv.clone()),
        SceneTtsBackend::Voicevox(voicevox) => run_voicevox_tts_backend(request, &config, voicevox),
    }
}

fn run_voicevox_tts_backend(
    request: SceneTtsRequest,
    config: &SceneTtsConfig,
    voicevox: &SceneVoicevoxConfig,
) -> SceneTtsResult {
    let segment = request.segment.clone();
    let output_path = tts_output_path(&config.cache_dir);
    let text = match translate_text(&request.segment.text, &voicevox.translation, config.timeout) {
        Ok(text) => text,
        Err(err) => {
            return finished_tts_result(segment, "TTS failed", None, Some(err));
        }
    };
    let query_path = format!(
        "/audio_query?speaker={}&text={}",
        voicevox.speaker,
        percent_encode_query_value(&text)
    );
    let query_json = match voicevox_http_request(
        voicevox,
        &query_path,
        None,
        "application/json",
        config.timeout,
    ) {
        Ok(body) => body,
        Err(err) => {
            return finished_tts_result(
                segment,
                "TTS failed",
                None,
                Some(format!("VOICEVOX audio_query failed: {err}")),
            );
        }
    };
    if serde_json::from_slice::<serde_json::Value>(&query_json).is_err() {
        return finished_tts_result(
            segment,
            "TTS failed",
            None,
            Some("VOICEVOX audio_query returned invalid JSON".to_string()),
        );
    }

    let synthesis_path = format!("/synthesis?speaker={}", voicevox.speaker);
    let wav = match voicevox_http_request(
        voicevox,
        &synthesis_path,
        Some(&query_json),
        "application/json",
        config.timeout,
    ) {
        Ok(body) => body,
        Err(err) => {
            return finished_tts_result(
                segment,
                "TTS failed",
                None,
                Some(format!("VOICEVOX synthesis failed: {err}")),
            );
        }
    };
    if wav.is_empty() {
        return finished_tts_result(
            segment,
            "TTS failed",
            None,
            Some("VOICEVOX synthesis produced empty audio".to_string()),
        );
    }

    if let Err(err) = std::fs::write(&output_path, wav) {
        return finished_tts_result(
            segment,
            "TTS failed",
            None,
            Some(format!("failed to write TTS output: {err}")),
        );
    }

    let played_audio = if let Some(player) = &config.player {
        if let Err(err) = run_player_command(player.clone(), &output_path, config.timeout) {
            let _ = std::fs::remove_file(&output_path);
            return finished_tts_result(
                segment,
                "TTS failed",
                None,
                Some(format!("TTS player failed: {err}")),
            );
        }
        let _ = std::fs::remove_file(&output_path);
        true
    } else {
        false
    };

    finished_tts_result(
        segment,
        if played_audio {
            "TTS played".to_string()
        } else {
            format!("TTS ready: {}", output_path.display())
        },
        (!played_audio).then_some(output_path),
        None,
    )
}

fn translate_text(
    text: &str,
    translation: &SceneTranslationConfig,
    timeout: Duration,
) -> Result<String, String> {
    match translation {
        SceneTranslationConfig::Off => Ok(text.to_string()),
        SceneTranslationConfig::Command(argv) => {
            run_translation_command(text, argv.clone(), timeout)
        }
    }
}

fn run_translation_command(
    text: &str,
    argv: Vec<String>,
    timeout: Duration,
) -> Result<String, String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("empty translation command".to_string());
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to spawn translation command `{program}`: {err}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(text.as_bytes());
    }
    drop(child.stdin.take());

    let output = wait_for_tts_output(child, timeout)?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "translation command failed".to_string()
        } else {
            error
        });
    }
    let translated = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if translated.is_empty() {
        Err("translation returned empty text".to_string())
    } else {
        Ok(translated)
    }
}

fn run_command_tts_backend(
    request: SceneTtsRequest,
    config: &SceneTtsConfig,
    argv: Vec<String>,
) -> SceneTtsResult {
    let segment = request.segment.clone();
    let output_path = tts_output_path(&config.cache_dir);
    let argv = argv
        .into_iter()
        .map(|arg| arg.replace("{output}", &output_path.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = argv.split_first() else {
        return finished_tts_result(
            segment,
            "TTS failed",
            None,
            Some("empty TTS command".to_string()),
        );
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
            return finished_tts_result(
                segment,
                "TTS failed",
                None,
                Some(format!("failed to spawn TTS command `{program}`: {err}")),
            );
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(request.segment.text.as_bytes());
    }
    drop(child.stdin.take());

    let output = match wait_for_tts_output(child, config.timeout) {
        Ok(output) => output,
        Err(err) => {
            return finished_tts_result(segment, "TTS failed", None, Some(err));
        }
    };

    if !output.status.success() {
        return finished_tts_result(
            segment,
            "TTS failed",
            None,
            Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        );
    }

    let played_audio = if let Some(player) = &config.player {
        if let Err(err) = run_player_command(player.clone(), &output_path, config.timeout) {
            let _ = std::fs::remove_file(&output_path);
            return finished_tts_result(
                segment,
                "TTS failed",
                None,
                Some(format!("TTS player failed: {err}")),
            );
        }
        let _ = std::fs::remove_file(&output_path);
        true
    } else {
        false
    };

    finished_tts_result(
        segment,
        if played_audio {
            "TTS played".to_string()
        } else {
            format!("TTS ready: {}", output_path.display())
        },
        (!played_audio).then_some(output_path),
        None,
    )
}

fn voicevox_http_request(
    voicevox: &SceneVoicevoxConfig,
    path_and_query: &str,
    body: Option<&[u8]>,
    content_type: &str,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let address = (voicevox.host.as_str(), voicevox.port)
        .to_socket_addrs()
        .map_err(|err| format!("failed to resolve VOICEVOX host: {err}"))?
        .next()
        .ok_or_else(|| "VOICEVOX host resolved to no addresses".to_string())?;
    let mut stream = TcpStream::connect_timeout(&address, timeout).map_err(|err| {
        format!(
            "VOICEVOX engine not reachable at {}:{}: {err}",
            voicevox.host, voicevox.port
        )
    })?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let body = body.unwrap_or(&[]);
    let request = format!(
        "POST {path_and_query} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
        voicevox.host,
        voicevox.port,
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|err| format!("failed to write VOICEVOX request: {err}"))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| format!("failed to read VOICEVOX response: {err}"))?;
    split_http_response(&response)
}

fn split_http_response(response: &[u8]) -> Result<Vec<u8>, String> {
    let Some(header_end) = response.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("VOICEVOX returned malformed HTTP response".to_string());
    };
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status_line = headers.lines().next().unwrap_or_default();
    if !status_line.contains(" 200 ") {
        return Err(status_line.trim().to_string());
    }
    Ok(response[header_end + 4..].to_vec())
}

fn percent_encode_query_value(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn env_first_non_empty(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn ct2_translation_command() -> Option<Vec<String>> {
    if let Some(command) = env_first_non_empty(&[TTS_CT2_COMMAND_ENV]) {
        return parse_command_argv(&command)
            .ok()
            .filter(|argv| !argv.is_empty());
    }

    let candidate = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("ci/scene-tts/ct2-en-to-ja.sh"))
        .filter(|path| path.exists())
        .or_else(|| {
            Some(PathBuf::from(
                "/Users/julianabeleda/env/gameterm/ci/scene-tts/ct2-en-to-ja.sh",
            ))
            .filter(|path| path.exists())
        })?;

    if Command::new(&candidate)
        .arg("--ready")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success())
    {
        Some(vec![candidate.display().to_string()])
    } else {
        None
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

fn run_player_command(
    argv: Vec<String>,
    output_path: &PathBuf,
    timeout: Duration,
) -> Result<(), String> {
    let argv = argv
        .into_iter()
        .map(|arg| arg.replace("{output}", &output_path.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = argv.split_first() else {
        return Ok(());
    };
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| err.to_string())?;
    let output = wait_for_tts_output(child, timeout)?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            "player command failed".to_string()
        } else {
            error
        })
    }
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
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

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
                    turn_id: 0,
                    block_index: 0,
                    speaker: Some("Codex".to_string()),
                    display_text: "Speak this line.".to_string(),
                    text: "Speak this line.".to_string(),
                    kind: SpeechBlockKind::Prose,
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
    fn visual_tts_worker_processes_queued_requests() {
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
        let (tx, rx) = mpsc::channel();
        let worker = SceneTtsWorker::new(
            SceneTtsConfig {
                backend: SceneTtsBackend::Command(vec![helper.display().to_string()]),
                player: None,
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
            tx,
        );

        worker.speak(SceneTtsRequest {
            segment: test_segment("first line"),
        });
        worker.speak(SceneTtsRequest {
            segment: test_segment("second line"),
        });

        let first_started = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let first = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let second_started = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let second = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(first_started.event, SceneTtsEvent::Started);
        assert_eq!(first_started.segment.text, "first line");
        assert_eq!(first.event, SceneTtsEvent::Finished);
        assert_eq!(second_started.event, SceneTtsEvent::Started);
        assert_eq!(second_started.segment.text, "second line");
        assert_eq!(second.event, SceneTtsEvent::Finished);
        assert!(first.succeeded());
        assert!(second.succeeded());
        assert_eq!(
            std::fs::read_to_string(first.output_path.unwrap()).unwrap(),
            "first line"
        );
        assert_eq!(
            std::fs::read_to_string(second.output_path.unwrap()).unwrap(),
            "second line"
        );
    }

    #[test]
    fn visual_tts_worker_waits_for_player_before_next_block() {
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("tts-helper.sh");
        let player = dir.path().join("player-helper.sh");
        let log = dir.path().join("player.log");
        std::fs::write(
            &helper,
            "#!/usr/bin/env sh\ncat > \"$GAMETERM_SCENE_TTS_OUTPUT\"\n",
        )
        .unwrap();
        std::fs::write(
            &player,
            format!(
                "#!/usr/bin/env sh\npayload=$(cat \"$1\")\nprintf 'start:%s\\n' \"$payload\" >> '{}'\nsleep 0.1\nprintf 'end:%s\\n' \"$payload\" >> '{}'\n",
                log.display(),
                log.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&helper, &player] {
                let mut permissions = std::fs::metadata(path).unwrap().permissions();
                permissions.set_mode(0o755);
                std::fs::set_permissions(path, permissions).unwrap();
            }
        }
        let (tx, rx) = mpsc::channel();
        let worker = SceneTtsWorker::new(
            SceneTtsConfig {
                backend: SceneTtsBackend::Command(vec![helper.display().to_string()]),
                player: Some(vec![player.display().to_string(), "{output}".to_string()]),
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
            tx,
        );

        worker.speak(SceneTtsRequest {
            segment: test_segment("first line"),
        });
        worker.speak(SceneTtsRequest {
            segment: test_segment("second line"),
        });

        let first_started = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let first = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let second_started = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let second = rx.recv_timeout(Duration::from_secs(3)).unwrap();

        assert_eq!(first_started.event, SceneTtsEvent::Started);
        assert_eq!(first_started.segment.text, "first line");
        assert_eq!(first.event, SceneTtsEvent::Finished);
        assert_eq!(second_started.event, SceneTtsEvent::Started);
        assert_eq!(second_started.segment.text, "second line");
        assert_eq!(second.event, SceneTtsEvent::Finished);
        assert!(first.succeeded());
        assert!(second.succeeded());
        assert_eq!(first.status, "TTS played");
        assert_eq!(second.status, "TTS played");
        assert!(first.output_path.is_none());
        assert!(second.output_path.is_none());
        assert_eq!(
            std::fs::read_to_string(log).unwrap(),
            "start:first line\nend:first line\nstart:second line\nend:second line\n"
        );
    }

    #[test]
    fn visual_tts_voicevox_backend_writes_wav_from_http() {
        let server = FakeVoicevoxServer::start();
        let dir = tempfile::tempdir().unwrap();
        let result = run_tts_backend(
            SceneTtsRequest {
                segment: test_segment("hello"),
            },
            SceneTtsConfig {
                backend: SceneTtsBackend::Voicevox(SceneVoicevoxConfig {
                    host: "127.0.0.1".to_string(),
                    port: server.port,
                    speaker: 14,
                    translation: SceneTranslationConfig::Off,
                }),
                player: None,
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
        );

        assert!(result.succeeded(), "{result:?}", result = result);
        let output_path = result.output_path.unwrap();
        assert_eq!(std::fs::read(output_path).unwrap(), b"RIFFvoicevox");
        let paths = server.requests();
        assert_eq!(paths[0].0, "/audio_query?speaker=14&text=hello");
        assert_eq!(paths[1].0, "/synthesis?speaker=14");
        assert_eq!(paths[1].1, br#"{"query":"ok"}"#);
    }

    #[test]
    fn visual_tts_voicevox_backend_uses_translation_command() {
        let server = FakeVoicevoxServer::start();
        let dir = tempfile::tempdir().unwrap();
        let helper = dir.path().join("translate.sh");
        std::fs::write(&helper, "#!/usr/bin/env sh\nprintf 'こんにちは'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&helper).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&helper, permissions).unwrap();
        }

        let result = run_tts_backend(
            SceneTtsRequest {
                segment: test_segment("hello"),
            },
            SceneTtsConfig {
                backend: SceneTtsBackend::Voicevox(SceneVoicevoxConfig {
                    host: "127.0.0.1".to_string(),
                    port: server.port,
                    speaker: 14,
                    translation: SceneTranslationConfig::Command(vec![helper
                        .display()
                        .to_string()]),
                }),
                player: None,
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
        );

        assert!(result.succeeded(), "{result:?}", result = result);
        let paths = server.requests();
        assert_eq!(
            paths[0].0,
            "/audio_query?speaker=14&text=%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF"
        );
    }

    #[test]
    fn visual_tts_state_toggles_mute() {
        let mut state = SceneTtsState::default();
        assert!(!state.is_muted());
        assert_eq!(state.toggle_muted(), "TTS muted");
        assert!(state.is_muted());
    }

    #[test]
    fn visual_tts_config_reports_playable_audio_backends() {
        let dir = tempfile::tempdir().unwrap();
        let disabled = SceneTtsConfig {
            backend: SceneTtsBackend::Disabled,
            player: None,
            cache_dir: dir.path().to_path_buf(),
            timeout: Duration::from_secs(2),
        };
        let silent = SceneTtsConfig {
            backend: SceneTtsBackend::BuiltInSilent,
            player: None,
            cache_dir: dir.path().to_path_buf(),
            timeout: Duration::from_secs(2),
        };
        let command = SceneTtsConfig {
            backend: SceneTtsBackend::Command(vec!["tts-helper".to_string()]),
            player: None,
            cache_dir: dir.path().to_path_buf(),
            timeout: Duration::from_secs(2),
        };
        let voicevox = SceneTtsConfig {
            backend: SceneTtsBackend::Voicevox(SceneVoicevoxConfig {
                host: "127.0.0.1".to_string(),
                port: 50021,
                speaker: 14,
                translation: SceneTranslationConfig::Off,
            }),
            player: None,
            cache_dir: dir.path().to_path_buf(),
            timeout: Duration::from_secs(2),
        };

        assert!(!disabled.can_play_audio());
        assert!(!silent.can_play_audio());
        assert!(command.can_play_audio());
        assert!(voicevox.can_play_audio());
    }

    fn test_segment(text: &str) -> SpeakableSegment {
        SpeakableSegment {
            turn_id: 0,
            block_index: 0,
            speaker: Some("Codex".to_string()),
            display_text: text.to_string(),
            text: text.to_string(),
            kind: SpeechBlockKind::Prose,
            source: SpeakableSource::ComposeReply,
        }
    }

    struct FakeVoicevoxServer {
        port: u16,
        rx: mpsc::Receiver<(String, Vec<u8>)>,
    }

    impl FakeVoicevoxServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                for index in 0..2 {
                    let (mut stream, _) = listener.accept().unwrap();
                    let (path, body) = read_test_http_request(&mut stream);
                    tx.send((path, body)).unwrap();
                    if index == 0 {
                        write_test_http_response(
                            &mut stream,
                            b"{\"query\":\"ok\"}",
                            "application/json",
                        );
                    } else {
                        write_test_http_response(&mut stream, b"RIFFvoicevox", "audio/wav");
                    }
                }
            });
            Self { port, rx }
        }

        fn requests(self) -> Vec<(String, Vec<u8>)> {
            vec![
                self.rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                self.rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ]
        }
    }

    fn read_test_http_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or_default()
            .to_string();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("Content-Length: ") {
                content_length = value.parse().unwrap();
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        (path, body)
    }

    fn write_test_http_response(stream: &mut TcpStream, body: &[u8], content_type: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
    }
}
