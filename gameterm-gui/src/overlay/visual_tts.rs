use super::visual_speech_blocks::SpeakableSegment;
#[cfg(test)]
use super::visual_speech_blocks::{SpeakableSource, SpeechBlockKind};
use super::visual_voice_trace::{trace_voice_event, SceneVoiceTraceEvent};
use std::io::Read;
use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
    pub(super) generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsResult {
    pub(super) event: SceneTtsEvent,
    pub(super) segment: SpeakableSegment,
    pub(super) generation: u64,
    pub(super) status: String,
    pub(super) output_path: Option<PathBuf>,
    pub(super) error: Option<String>,
    pub(super) timing: SceneTtsTiming,
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
    request: &SceneTtsRequest,
    status: impl Into<String>,
    output_path: Option<PathBuf>,
    error: Option<String>,
    timing: SceneTtsTiming,
) -> SceneTtsResult {
    SceneTtsResult {
        event: SceneTtsEvent::Finished,
        segment: request.segment.clone(),
        generation: request.generation,
        status: status.into(),
        output_path,
        error,
        timing,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct SceneTtsTiming {
    pub(super) translation_ms: Option<u128>,
    pub(super) query_ms: Option<u128>,
    pub(super) synthesis_ms: Option<u128>,
    pub(super) player_ms: Option<u128>,
    pub(super) total_ms: Option<u128>,
}

impl SceneTtsTiming {
    pub(super) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ms) = self.translation_ms {
            parts.push(format!("translate={ms}ms"));
        }
        if let Some(ms) = self.query_ms {
            parts.push(format!("query={ms}ms"));
        }
        if let Some(ms) = self.synthesis_ms {
            parts.push(format!("synth={ms}ms"));
        }
        if let Some(ms) = self.player_ms {
            parts.push(format!("play={ms}ms"));
        }
        if let Some(ms) = self.total_ms {
            parts.push(format!("total={ms}ms"));
        }
        if parts.is_empty() {
            "timing unavailable".to_string()
        } else {
            parts.join(", ")
        }
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
    /// Translation was requested (implicitly via the default/ct2 backend, or
    /// explicitly via a backend/command) but no usable translator is available.
    /// Carries a human-readable reason so the absence is surfaced through the
    /// normal TTS error path instead of silently degrading to untranslated text.
    Unavailable { reason: String },
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
    pub(crate) fn built_in_silent_for_test() -> Self {
        Self {
            backend: SceneTtsBackend::BuiltInSilent,
            player: None,
            cache_dir: std::env::temp_dir(),
            timeout: Duration::from_secs(2),
        }
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
                .unwrap_or_else(|| SceneTranslationConfig::Unavailable {
                    reason: format!(
                        "{} is set but could not be parsed into a command",
                        TTS_TRANSLATION_COMMAND_ENV
                    ),
                });
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
                .unwrap_or_else(|| SceneTranslationConfig::Unavailable {
                    reason: format!(
                        "{}=command but {} is unset or invalid",
                        TTS_TRANSLATION_BACKEND_ENV, TTS_TRANSLATION_COMMAND_ENV
                    ),
                }),
            // Default (unset) and explicit `ct2` both auto-discover the optional
            // CT2 translator. When it isn't installed, surface that fact instead
            // of silently collapsing to `Off` (which speaks untranslated text).
            Some("ct2") | None => ct2_translation_command()
                .map(SceneTranslationConfig::Command)
                .unwrap_or_else(|| SceneTranslationConfig::Unavailable {
                    reason: format!(
                        "CT2 translator not installed; run ci/scene-tts/setup-ct2-en-ja.sh, \
                         or set {} / {}. Set {}=off to speak untranslated text intentionally.",
                        TTS_CT2_COMMAND_ENV, TTS_TRANSLATION_COMMAND_ENV, TTS_TRANSLATION_BACKEND_ENV
                    ),
                }),
            Some(other) => SceneTranslationConfig::Unavailable {
                reason: format!("unknown {} value `{}`", TTS_TRANSLATION_BACKEND_ENV, other),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTtsState {
    muted: bool,
    generation: u64,
    queued_blocks: usize,
    current: Option<SceneTtsCurrentBlock>,
    last_status: String,
    last_timing: Option<String>,
    last_error: Option<String>,
    last_skipped: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneTtsCurrentBlock {
    turn_id: u64,
    block_index: usize,
    phase: &'static str,
    preview: String,
}

impl Default for SceneTtsState {
    fn default() -> Self {
        Self {
            muted: false,
            generation: 1,
            queued_blocks: 0,
            current: None,
            last_status: "TTS idle".to_string(),
            last_timing: None,
            last_error: None,
            last_skipped: None,
        }
    }
}

impl SceneTtsState {
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn begin_new_generation(&mut self) -> String {
        self.generation = self.generation.saturating_add(1);
        self.queued_blocks = 0;
        self.current = None;
        self.last_skipped = Some("previous speech queue invalidated".to_string());
        self.last_status = "TTS queue reset".to_string();
        self.last_status.clone()
    }

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

    pub(super) fn mark_queued(&mut self, count: usize) -> String {
        if count == 0 {
            self.last_status = "TTS no speakable blocks".to_string();
        } else {
            self.queued_blocks = self.queued_blocks.saturating_add(count);
            self.last_status = format!("TTS queued: {count} block(s)");
        }
        self.last_status.clone()
    }

    pub(super) fn accepts_result(&self, result: &SceneTtsResult) -> bool {
        result.generation == self.generation
    }

    pub(super) fn apply_result(&mut self, result: &SceneTtsResult) -> String {
        if !self.accepts_result(result) {
            self.last_skipped = Some(format!(
                "stale TTS event ignored: result={} current={}",
                result.generation, self.generation
            ));
            self.last_status = "TTS stale event ignored".to_string();
            return self.last_status.clone();
        }

        match result.event {
            SceneTtsEvent::Started => {
                self.queued_blocks = self.queued_blocks.saturating_sub(1);
                self.current = Some(SceneTtsCurrentBlock {
                    turn_id: result.segment.turn_id,
                    block_index: result.segment.block_index,
                    phase: "synthesizing",
                    preview: result.segment.text.chars().take(60).collect(),
                });
            }
            SceneTtsEvent::Finished => {
                self.current = None;
                self.last_timing = result.timing.total_ms.map(|_| result.timing.summary());
                self.last_error = result.error.clone();
            }
        }
        self.last_status = result.status.clone();
        self.last_status.clone()
    }

    pub(super) fn diagnostics_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!(
            "TTS: {}",
            if self.muted { "muted" } else { "unmuted" }
        ));
        lines.push(format!("TTS queue generation: {}", self.generation));
        lines.push(format!("TTS queued blocks: {}", self.queued_blocks));
        if let Some(current) = self.current.as_ref() {
            lines.push(format!(
                "TTS current: turn {} block {} {}",
                current.turn_id, current.block_index, current.phase
            ));
            lines.push(format!("TTS current text: {}", current.preview));
        } else {
            lines.push("TTS current: idle".to_string());
        }
        if let Some(timing) = self.last_timing.as_deref() {
            lines.push(format!("TTS last timing: {timing}"));
        }
        if let Some(skipped) = self.last_skipped.as_deref() {
            lines.push(format!("TTS last skipped: {skipped}"));
        }
        if let Some(error) = self.last_error.as_deref() {
            lines.push(format!("TTS last error: {error}"));
        }
        lines
    }
}

enum SceneTtsWorkerMessage {
    Speak(SceneTtsRequest),
    Shutdown,
}

pub(super) struct SceneTtsWorker {
    tx: mpsc::Sender<SceneTtsWorkerMessage>,
    active_generation: Arc<AtomicU64>,
}

impl SceneTtsWorker {
    pub(super) fn new(config: SceneTtsConfig, result_tx: mpsc::Sender<SceneTtsResult>) -> Self {
        let (tx, rx) = mpsc::channel();
        let active_generation = Arc::new(AtomicU64::new(1));
        let worker_generation = active_generation.clone();
        thread::spawn(move || {
            while let Ok(message) = rx.recv() {
                match message {
                    SceneTtsWorkerMessage::Speak(request) => {
                        if request_is_stale(&request, &worker_generation) {
                            let result = finished_tts_result(
                                &request,
                                "TTS skipped stale block",
                                None,
                                Some("speech queue generation changed".to_string()),
                                SceneTtsTiming::default(),
                            );
                            trace_tts_request_event(
                                "tts_worker_skipped_stale_request",
                                &request,
                                Some("TTS skipped stale block"),
                                Some("speech queue generation changed"),
                                None,
                            );
                            let _ = result_tx.send(result);
                            continue;
                        }
                        trace_tts_request_event(
                            "tts_worker_started",
                            &request,
                            Some("TTS speaking"),
                            None,
                            None,
                        );
                        let _ = result_tx.send(SceneTtsResult {
                            event: SceneTtsEvent::Started,
                            segment: request.segment.clone(),
                            generation: request.generation,
                            status: "TTS speaking".to_string(),
                            output_path: None,
                            error: None,
                            timing: SceneTtsTiming::default(),
                        });
                        let result =
                            run_tts_backend(request, config.clone(), worker_generation.clone());
                        let _ = result_tx.send(result);
                    }
                    SceneTtsWorkerMessage::Shutdown => break,
                }
            }
        });
        Self {
            tx,
            active_generation,
        }
    }

    pub(super) fn speak(&self, request: SceneTtsRequest) {
        let _ = self.tx.send(SceneTtsWorkerMessage::Speak(request));
    }

    pub(super) fn set_generation(&self, generation: u64) {
        self.active_generation.store(generation, Ordering::SeqCst);
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

fn run_tts_backend(
    request: SceneTtsRequest,
    config: SceneTtsConfig,
    active_generation: Arc<AtomicU64>,
) -> SceneTtsResult {
    let total_started = Instant::now();
    match &config.backend {
        SceneTtsBackend::Disabled => finished_tts_result(
            &request,
            "TTS disabled",
            None,
            None,
            timing_with_total(SceneTtsTiming::default(), total_started),
        ),
        SceneTtsBackend::BuiltInSilent => finished_tts_result(
            &request,
            format!("TTS silent: {}", request.segment.text),
            None,
            None,
            timing_with_total(SceneTtsTiming::default(), total_started),
        ),
        SceneTtsBackend::Command(argv) => run_command_tts_backend(
            request,
            &config,
            argv.clone(),
            active_generation,
            total_started,
        ),
        SceneTtsBackend::Voicevox(voicevox) => {
            run_voicevox_tts_backend(request, &config, voicevox, active_generation, total_started)
        }
    }
}

fn run_voicevox_tts_backend(
    request: SceneTtsRequest,
    config: &SceneTtsConfig,
    voicevox: &SceneVoicevoxConfig,
    active_generation: Arc<AtomicU64>,
    total_started: Instant,
) -> SceneTtsResult {
    let mut timing = SceneTtsTiming::default();
    let output_path = tts_output_path(&config.cache_dir);
    let translation_started = Instant::now();
    let text = match translate_text(
        &request.segment.text,
        &voicevox.translation,
        config.timeout,
        &active_generation,
        request.generation,
    ) {
        Ok(text) => text,
        Err(err) => {
            timing.translation_ms = Some(translation_started.elapsed().as_millis());
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(err),
                timing_with_total(timing, total_started),
            );
        }
    };
    timing.translation_ms = Some(translation_started.elapsed().as_millis());
    let query_path = format!(
        "/audio_query?speaker={}&text={}",
        voicevox.speaker,
        percent_encode_query_value(&text)
    );
    trace_tts_request_event(
        "voicevox_audio_query_start",
        &request,
        None,
        None,
        Some(serde_json::json!({
            "voicevox_host": voicevox.host,
            "voicevox_port": voicevox.port,
            "speaker_id": voicevox.speaker,
        })),
    );
    let query_started = Instant::now();
    let query_json = match voicevox_http_request(
        voicevox,
        &query_path,
        None,
        "application/json",
        config.timeout,
    ) {
        Ok(body) => {
            timing.query_ms = Some(query_started.elapsed().as_millis());
            trace_tts_request_event(
                "voicevox_audio_query_success",
                &request,
                Some("VOICEVOX audio_query succeeded"),
                None,
                Some(serde_json::json!({
                    "query_ms": timing.query_ms,
                    "response_bytes": body.len(),
                })),
            );
            body
        }
        Err(err) => {
            trace_tts_request_event(
                "voicevox_audio_query_failure",
                &request,
                Some("VOICEVOX audio_query failed"),
                Some(&err),
                Some(serde_json::json!({
                    "query_ms": query_started.elapsed().as_millis(),
                })),
            );
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(format!("VOICEVOX audio_query failed: {err}")),
                timing_with_total(timing, total_started),
            );
        }
    };
    if serde_json::from_slice::<serde_json::Value>(&query_json).is_err() {
        trace_tts_request_event(
            "voicevox_audio_query_failure",
            &request,
            Some("VOICEVOX audio_query returned invalid JSON"),
            Some("VOICEVOX audio_query returned invalid JSON"),
            None,
        );
        return finished_tts_result(
            &request,
            "TTS failed",
            None,
            Some("VOICEVOX audio_query returned invalid JSON".to_string()),
            timing_with_total(timing, total_started),
        );
    }

    let synthesis_path = format!("/synthesis?speaker={}", voicevox.speaker);
    trace_tts_request_event(
        "voicevox_synthesis_start",
        &request,
        None,
        None,
        Some(serde_json::json!({
            "speaker_id": voicevox.speaker,
        })),
    );
    let synthesis_started = Instant::now();
    let wav = match voicevox_http_request(
        voicevox,
        &synthesis_path,
        Some(&query_json),
        "application/json",
        config.timeout,
    ) {
        Ok(body) => {
            timing.synthesis_ms = Some(synthesis_started.elapsed().as_millis());
            trace_tts_request_event(
                "voicevox_synthesis_success",
                &request,
                Some("VOICEVOX synthesis succeeded"),
                None,
                Some(serde_json::json!({
                    "synthesis_ms": timing.synthesis_ms,
                    "wav_bytes": body.len(),
                })),
            );
            body
        }
        Err(err) => {
            trace_tts_request_event(
                "voicevox_synthesis_failure",
                &request,
                Some("VOICEVOX synthesis failed"),
                Some(&err),
                Some(serde_json::json!({
                    "synthesis_ms": synthesis_started.elapsed().as_millis(),
                })),
            );
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(format!("VOICEVOX synthesis failed: {err}")),
                timing_with_total(timing, total_started),
            );
        }
    };
    if wav.is_empty() {
        trace_tts_request_event(
            "voicevox_synthesis_failure",
            &request,
            Some("VOICEVOX synthesis produced empty audio"),
            Some("VOICEVOX synthesis produced empty audio"),
            None,
        );
        return finished_tts_result(
            &request,
            "TTS failed",
            None,
            Some("VOICEVOX synthesis produced empty audio".to_string()),
            timing_with_total(timing, total_started),
        );
    }

    if let Err(err) = std::fs::write(&output_path, wav) {
        trace_tts_request_event(
            "wav_write_failure",
            &request,
            Some("failed to write TTS output"),
            Some(&err.to_string()),
            Some(serde_json::json!({
                "output_path": output_path.display().to_string(),
            })),
        );
        return finished_tts_result(
            &request,
            "TTS failed",
            None,
            Some(format!("failed to write TTS output: {err}")),
            timing_with_total(timing, total_started),
        );
    }
    trace_tts_request_event(
        "wav_write_success",
        &request,
        Some("TTS output written"),
        None,
        Some(serde_json::json!({
            "output_path": output_path.display().to_string(),
        })),
    );

    let played_audio = if let Some(player) = &config.player {
        let player_started = Instant::now();
        if let Err(err) = run_player_command(
            &request,
            player.clone(),
            &output_path,
            config.timeout,
            &active_generation,
            request.generation,
        ) {
            let _ = std::fs::remove_file(&output_path);
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(format!("TTS player failed: {err}")),
                timing_with_total(timing, total_started),
            );
        }
        timing.player_ms = Some(player_started.elapsed().as_millis());
        let _ = std::fs::remove_file(&output_path);
        true
    } else {
        false
    };

    finished_tts_result(
        &request,
        if played_audio {
            "TTS played".to_string()
        } else {
            format!("TTS ready: {}", output_path.display())
        },
        (!played_audio).then_some(output_path),
        None,
        timing_with_total(timing, total_started),
    )
}

fn translate_text(
    text: &str,
    translation: &SceneTranslationConfig,
    timeout: Duration,
    active_generation: &Arc<AtomicU64>,
    generation: u64,
) -> Result<String, String> {
    match translation {
        SceneTranslationConfig::Off => Ok(text.to_string()),
        SceneTranslationConfig::Command(argv) => {
            run_translation_command(text, argv.clone(), timeout, active_generation, generation)
        }
        // Requested but no translator available: surface it through the normal
        // TTS error path rather than speaking untranslated text.
        SceneTranslationConfig::Unavailable { reason } => Err(reason.clone()),
    }
}

fn run_translation_command(
    text: &str,
    argv: Vec<String>,
    timeout: Duration,
    active_generation: &Arc<AtomicU64>,
    generation: u64,
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

    let output = wait_for_tts_output(child, timeout, active_generation, generation)?;
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
    active_generation: Arc<AtomicU64>,
    total_started: Instant,
) -> SceneTtsResult {
    let mut timing = SceneTtsTiming::default();
    let output_path = tts_output_path(&config.cache_dir);
    let argv = argv
        .into_iter()
        .map(|arg| arg.replace("{output}", &output_path.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = argv.split_first() else {
        return finished_tts_result(
            &request,
            "TTS failed",
            None,
            Some("empty TTS command".to_string()),
            timing_with_total(timing, total_started),
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
                &request,
                "TTS failed",
                None,
                Some(format!("failed to spawn TTS command `{program}`: {err}")),
                timing_with_total(timing, total_started),
            );
        }
    };

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(request.segment.text.as_bytes());
    }
    drop(child.stdin.take());

    let synthesis_started = Instant::now();
    let output = match wait_for_tts_output(
        child,
        config.timeout,
        &active_generation,
        request.generation,
    ) {
        Ok(output) => {
            timing.synthesis_ms = Some(synthesis_started.elapsed().as_millis());
            output
        }
        Err(err) => {
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(err),
                timing_with_total(timing, total_started),
            );
        }
    };

    if !output.status.success() {
        return finished_tts_result(
            &request,
            "TTS failed",
            None,
            Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            timing_with_total(timing, total_started),
        );
    }

    let played_audio = if let Some(player) = &config.player {
        let player_started = Instant::now();
        if let Err(err) = run_player_command(
            &request,
            player.clone(),
            &output_path,
            config.timeout,
            &active_generation,
            request.generation,
        ) {
            let _ = std::fs::remove_file(&output_path);
            return finished_tts_result(
                &request,
                "TTS failed",
                None,
                Some(format!("TTS player failed: {err}")),
                timing_with_total(timing, total_started),
            );
        }
        timing.player_ms = Some(player_started.elapsed().as_millis());
        let _ = std::fs::remove_file(&output_path);
        true
    } else {
        false
    };

    finished_tts_result(
        &request,
        if played_audio {
            "TTS played".to_string()
        } else {
            format!("TTS ready: {}", output_path.display())
        },
        (!played_audio).then_some(output_path),
        None,
        timing_with_total(timing, total_started),
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

fn request_is_stale(request: &SceneTtsRequest, active_generation: &Arc<AtomicU64>) -> bool {
    !generation_is_current(active_generation, request.generation)
}

fn generation_is_current(active_generation: &Arc<AtomicU64>, generation: u64) -> bool {
    active_generation.load(Ordering::SeqCst) == generation
}

fn timing_with_total(mut timing: SceneTtsTiming, started: Instant) -> SceneTtsTiming {
    timing.total_ms = Some(started.elapsed().as_millis());
    timing
}

fn ct2_translation_command() -> Option<Vec<String>> {
    if let Some(command) = env_first_non_empty(&[TTS_CT2_COMMAND_ENV]) {
        return parse_command_argv(&command)
            .ok()
            .filter(|argv| !argv.is_empty());
    }

    // Auto-discovery convenience only; GAMETERM_SCENE_TTS_CT2_COMMAND is the
    // canonical override. Looks in the current directory, then the default
    // checkout location under the user's home.
    let candidate = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("ci/scene-tts/ct2-en-to-ja.sh"))
        .filter(|path| path.exists())
        .or_else(|| {
            dirs_next::home_dir()
                .map(|home| home.join("env/gameterm/ci/scene-tts/ct2-en-to-ja.sh"))
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
    active_generation: &Arc<AtomicU64>,
    generation: u64,
) -> Result<std::process::Output, String> {
    let started = std::time::Instant::now();
    loop {
        if !generation_is_current(active_generation, generation) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("TTS stopped".to_string());
        }
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
    request: &SceneTtsRequest,
    argv: Vec<String>,
    output_path: &PathBuf,
    timeout: Duration,
    active_generation: &Arc<AtomicU64>,
    generation: u64,
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
    let player_pid = child.id();
    trace_tts_request_event(
        "player_spawn",
        request,
        Some("TTS player spawned"),
        None,
        Some(serde_json::json!({
            "output_path": output_path.display().to_string(),
            "player_argv": argv,
            "player_pid": player_pid,
        })),
    );
    let output = match wait_for_tts_output(child, timeout, active_generation, generation) {
        Ok(output) => output,
        Err(err) => {
            trace_tts_request_event(
                "player_failure",
                request,
                Some("TTS player failed"),
                Some(&err),
                Some(serde_json::json!({
                    "output_path": output_path.display().to_string(),
                    "player_pid": player_pid,
                })),
            );
            return Err(err);
        }
    };
    if output.status.success() {
        trace_tts_request_event(
            "player_success",
            request,
            Some("TTS player succeeded"),
            None,
            Some(serde_json::json!({
                "output_path": output_path.display().to_string(),
                "player_pid": player_pid,
            })),
        );
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        trace_tts_request_event(
            "player_failure",
            request,
            Some("TTS player failed"),
            Some(if error.is_empty() {
                "player command failed"
            } else {
                &error
            }),
            Some(serde_json::json!({
                "output_path": output_path.display().to_string(),
                "player_pid": player_pid,
            })),
        );
        Err(if error.is_empty() {
            "player command failed".to_string()
        } else {
            error
        })
    }
}

fn trace_tts_request_event(
    event: &'static str,
    request: &SceneTtsRequest,
    status: Option<&str>,
    error: Option<&str>,
    extra: Option<serde_json::Value>,
) {
    let mut trace = SceneVoiceTraceEvent::new(event).with_text(request.segment.text.clone());
    trace.turn_id = Some(request.segment.turn_id);
    trace.block_index = Some(request.segment.block_index);
    trace.generation = Some(request.generation);
    trace.speaker = request.segment.speaker.clone();
    trace.status = status.map(ToOwned::to_owned);
    trace.error = error.map(ToOwned::to_owned);
    if let Some(extra) = extra.as_ref() {
        trace.output_path = extra
            .get("output_path")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        trace.player_argv = extra
            .get("player_argv")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            });
        trace.player_pid = extra
            .get("player_pid")
            .and_then(serde_json::Value::as_u64)
            .filter(|pid| *pid <= u32::MAX as u64)
            .map(|pid| pid as u32);
    }
    trace.timing = extra;
    trace_voice_event(trace);
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
                generation: 1,
            },
            SceneTtsConfig {
                backend: SceneTtsBackend::Command(vec![helper.display().to_string()]),
                player: None,
                cache_dir: dir.path().to_path_buf(),
                timeout: Duration::from_secs(2),
            },
            test_active_generation(),
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

        worker.speak(test_request("first line"));
        worker.speak(test_request("second line"));

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

        worker.speak(test_request("first line"));
        worker.speak(test_request("second line"));

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
    fn visual_tts_worker_skips_requests_from_stale_generation() {
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

        worker.set_generation(2);
        worker.speak(test_request("old line"));

        let result = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(result.event, SceneTtsEvent::Finished);
        assert_eq!(result.status, "TTS skipped stale block");
        assert_eq!(result.generation, 1);
        assert_eq!(
            result.error.as_deref(),
            Some("speech queue generation changed")
        );
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());
    }

    #[test]
    fn visual_tts_voicevox_backend_writes_wav_from_http() {
        let server = FakeVoicevoxServer::start();
        let dir = tempfile::tempdir().unwrap();
        let result = run_tts_backend(
            test_request("hello"),
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
            test_active_generation(),
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
            test_request("hello"),
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
            test_active_generation(),
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
    fn visual_tts_state_ignores_stale_results_without_current_block() {
        let mut state = SceneTtsState::default();
        state.begin_new_generation();
        let result = finished_tts_result(
            &test_request("old line"),
            "TTS played",
            None,
            None,
            SceneTtsTiming {
                total_ms: Some(1),
                ..SceneTtsTiming::default()
            },
        );

        assert_eq!(state.apply_result(&result), "TTS stale event ignored");

        let diagnostics = state.diagnostics_lines().join("\n");
        assert!(diagnostics.contains("TTS current: idle"));
        assert!(diagnostics.contains("stale TTS event ignored"));
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

    fn test_request(text: &str) -> SceneTtsRequest {
        SceneTtsRequest {
            segment: test_segment(text),
            generation: 1,
        }
    }

    fn test_active_generation() -> Arc<AtomicU64> {
        Arc::new(AtomicU64::new(1))
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
