use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const STT_BACKEND_ENV: &str = "GAMETERM_SCENE_STT_BACKEND";
const STT_COMMAND_ENV: &str = "GAMETERM_SCENE_STT_COMMAND";
const STT_TIMEOUT_ENV: &str = "GAMETERM_SCENE_STT_TIMEOUT_SECONDS";
const STT_AUTO_SUBMIT_ENV: &str = "GAMETERM_SCENE_STT_AUTO_SUBMIT";
const STT_WHISPER_MODEL_ENV: &str = "GAMETERM_SCENE_STT_WHISPER_MODEL";
const STT_WHISPER_LANGUAGE_ENV: &str = "GAMETERM_SCENE_STT_LANGUAGE";
const STT_WHISPER_MAX_SECONDS_ENV: &str = "GAMETERM_SCENE_STT_MAX_SECONDS";
const STT_WHISPER_DEVICE_ENV: &str = "GAMETERM_SCENE_STT_DEVICE";
const STT_DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
const STT_DEFAULT_MAX_SECONDS: Duration = Duration::from_secs(20);
const MIC_TEST_DURATION: Duration = Duration::from_millis(900);
const STT_MAX_TRANSCRIPT_CHARS: usize = 800;
const WHISPER_SAMPLE_RATE: u32 = 16_000;

static WHISPER_CONTEXT_CACHE: LazyLock<Mutex<Option<(PathBuf, Arc<WhisperContext>)>>> =
    LazyLock::new(|| Mutex::new(None));

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
    Whisper(SceneWhisperConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SceneSttConfig {
    backend: SceneSttBackend,
    timeout: Duration,
    auto_submit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SceneWhisperConfig {
    model_path: PathBuf,
    language: String,
    max_recording: Duration,
    input_device: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneSttState {
    running: bool,
    last_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneMicDevice {
    pub(super) name: String,
    pub(super) is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SceneMicTestResult {
    pub(super) status: String,
    pub(super) device_label: String,
    pub(super) peak: Option<f32>,
    pub(super) rms: Option<f32>,
    pub(super) error: Option<String>,
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

    pub(super) fn mark_processing(&mut self) -> String {
        self.last_status = "Voice processing".to_string();
        self.last_status.clone()
    }

    pub(super) fn apply_result(&mut self, result: &SceneSttResult) -> String {
        self.running = false;
        self.last_status = result.status.clone();
        self.last_status.clone()
    }

    pub(super) fn last_status(&self) -> &str {
        &self.last_status
    }
}

#[derive(Debug)]
pub(super) struct SceneSttSession {
    tx: mpsc::Sender<SceneSttControl>,
}

impl SceneSttSession {
    pub(super) fn cancel(&self) {
        let _ = self.tx.send(SceneSttControl::Cancel);
    }

    pub(super) fn stop(&self) {
        let _ = self.tx.send(SceneSttControl::Stop);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SceneSttControl {
    Stop,
    Cancel,
}

pub(super) fn spawn_stt_backend(
    config: SceneSttConfig,
    tx: mpsc::Sender<SceneSttResult>,
) -> SceneSttSession {
    let (control_tx, control_rx) = mpsc::channel();
    thread::spawn(move || {
        let result = run_stt_backend(config, control_rx);
        let _ = tx.send(result);
    });
    SceneSttSession { tx: control_tx }
}

pub(super) fn spawn_mic_test(input_device: Option<String>, tx: mpsc::Sender<SceneMicTestResult>) {
    thread::spawn(move || {
        let result = run_mic_test(input_device.as_deref());
        let _ = tx.send(result);
    });
}

pub(super) fn scene_microphone_devices() -> Result<Vec<SceneMicDevice>, String> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .map_err(|err| format!("failed to list microphone input devices: {err}"))?;
    let mut out = Vec::new();
    for device in devices {
        let name = device.name().unwrap_or_else(|_| "<unnamed>".to_string());
        if out
            .iter()
            .any(|existing: &SceneMicDevice| existing.name == name)
        {
            continue;
        }
        let is_default = default_name
            .as_deref()
            .is_some_and(|default_name| default_name == name);
        out.push(SceneMicDevice { name, is_default });
    }
    Ok(out)
}

pub(crate) fn scene_stt_config_from_env() -> SceneSttConfig {
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
        Some("whisper") => SceneSttBackend::Whisper(SceneWhisperConfig::from_env()),
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

impl SceneSttConfig {
    pub(crate) fn from_env() -> Self {
        scene_stt_config_from_env()
    }

    pub(crate) fn whisper_default() -> Self {
        Self {
            backend: SceneSttBackend::Whisper(SceneWhisperConfig::from_env()),
            timeout: STT_DEFAULT_TIMEOUT,
            auto_submit: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn is_whisper_backend(&self) -> bool {
        matches!(self.backend, SceneSttBackend::Whisper(_))
    }

    pub(super) fn diagnostics_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("Backend: {}", self.backend_label()));
        lines.push(format!(
            "Auto submit: {}",
            if self.auto_submit { "on" } else { "off" }
        ));
        match &self.backend {
            SceneSttBackend::Whisper(whisper) => {
                lines.push(format!("Mic: {}", whisper.input_device_label()));
                lines.push(format!("Model: {}", whisper.model_path.display()));
                lines.push(format!("Language: {}", whisper.language));
                lines.push(format!(
                    "Max recording: {}s",
                    whisper.max_recording.as_secs()
                ));
            }
            SceneSttBackend::Command(argv) => {
                lines.push(format!("Command: {}", argv.join(" ")));
            }
            SceneSttBackend::Disabled => {}
        }
        lines
    }

    pub(super) fn with_input_device(&self, input_device: Option<String>) -> Self {
        let mut config = self.clone();
        if let SceneSttBackend::Whisper(whisper) = &mut config.backend {
            whisper.input_device = input_device;
        }
        config
    }

    fn backend_label(&self) -> &'static str {
        match self.backend {
            SceneSttBackend::Disabled => "disabled",
            SceneSttBackend::Command(_) => "command",
            SceneSttBackend::Whisper(_) => "whisper",
        }
    }
}

fn run_mic_test(configured_device: Option<&str>) -> SceneMicTestResult {
    let device_label = configured_device
        .map(str::trim)
        .filter(|device| !device.is_empty())
        .unwrap_or("system default")
        .to_string();
    match record_for_duration(MIC_TEST_DURATION, configured_device) {
        Ok(recording) => {
            let peak = recording
                .samples
                .iter()
                .copied()
                .fold(0.0_f32, |max, sample| max.max(sample.abs()));
            let rms = if recording.samples.is_empty() {
                0.0
            } else {
                let sum: f32 = recording.samples.iter().map(|sample| sample * sample).sum();
                (sum / recording.samples.len() as f32).sqrt()
            };
            let status = if recording.samples.is_empty() {
                "Mic test heard no samples".to_string()
            } else if peak >= 0.02 {
                format!("Mic signal detected: peak {peak:.3}")
            } else {
                format!("Mic silence: peak {peak:.3}")
            };
            SceneMicTestResult {
                status,
                device_label,
                peak: Some(peak),
                rms: Some(rms),
                error: None,
            }
        }
        Err(err) => SceneMicTestResult {
            status: "Mic test failed".to_string(),
            device_label,
            peak: None,
            rms: None,
            error: Some(err),
        },
    }
}

impl SceneWhisperConfig {
    fn from_env() -> Self {
        Self {
            model_path: std::env::var(STT_WHISPER_MODEL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(default_whisper_model_path),
            language: std::env::var(STT_WHISPER_LANGUAGE_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "en".to_string()),
            max_recording: std::env::var(STT_WHISPER_MAX_SECONDS_ENV)
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(STT_DEFAULT_MAX_SECONDS),
            input_device: std::env::var(STT_WHISPER_DEVICE_ENV)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        }
    }

    fn input_device_label(&self) -> String {
        self.input_device
            .as_deref()
            .filter(|device| !device.trim().is_empty())
            .unwrap_or("system default")
            .to_string()
    }
}

fn default_whisper_model_path() -> PathBuf {
    dirs_next::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("gameterm")
        .join("scene-stt")
        .join("models")
        .join("ggml-base.en.bin")
}

fn run_stt_backend(
    config: SceneSttConfig,
    control_rx: mpsc::Receiver<SceneSttControl>,
) -> SceneSttResult {
    match &config.backend {
        SceneSttBackend::Disabled => SceneSttResult {
            status: "Voice disabled".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some("STT backend is disabled".to_string()),
        },
        SceneSttBackend::Command(argv) => {
            run_command_stt_backend(&config, argv.clone(), control_rx)
        }
        SceneSttBackend::Whisper(whisper) => run_whisper_stt_backend(&config, whisper, control_rx),
    }
}

fn run_command_stt_backend(
    config: &SceneSttConfig,
    argv: Vec<String>,
    control_rx: mpsc::Receiver<SceneSttControl>,
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
        if matches!(control_rx.try_recv(), Ok(SceneSttControl::Cancel)) {
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

fn run_whisper_stt_backend(
    config: &SceneSttConfig,
    whisper: &SceneWhisperConfig,
    control_rx: mpsc::Receiver<SceneSttControl>,
) -> SceneSttResult {
    if !whisper.model_path.exists() {
        return SceneSttResult {
            status: "Voice failed".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some(format!(
                "missing Whisper model at {}",
                whisper.model_path.display()
            )),
        };
    }

    let recording = match record_until_stop(
        whisper.max_recording,
        whisper.input_device.as_deref(),
        control_rx,
    ) {
        Ok(Some(recording)) => recording,
        Ok(None) => {
            return SceneSttResult {
                status: "Voice canceled".to_string(),
                transcript: None,
                auto_submit: false,
                error: Some("STT canceled".to_string()),
            };
        }
        Err(err) => {
            return SceneSttResult {
                status: "Voice failed".to_string(),
                transcript: None,
                auto_submit: false,
                error: Some(err),
            };
        }
    };

    let samples = resample_linear_mono(
        &recording.samples,
        recording.sample_rate,
        WHISPER_SAMPLE_RATE,
    );
    if samples.len() < (WHISPER_SAMPLE_RATE / 4) as usize {
        return SceneSttResult {
            status: "Voice failed".to_string(),
            transcript: None,
            auto_submit: false,
            error: Some("recording too short".to_string()),
        };
    }

    let transcript =
        match transcribe_whisper_samples(&whisper.model_path, &whisper.language, &samples) {
            Ok(transcript) => sanitize_transcript(&transcript),
            Err(err) => {
                return SceneSttResult {
                    status: "Voice failed".to_string(),
                    transcript: None,
                    auto_submit: false,
                    error: Some(err),
                };
            }
        };
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

struct RecordedAudio {
    samples: Vec<f32>,
    sample_rate: u32,
}

struct ActiveRecording {
    stream: cpal::Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl ActiveRecording {
    fn finish(self) -> Result<RecordedAudio, String> {
        drop(self.stream);
        let samples = self
            .samples
            .lock()
            .map_err(|_| "failed to read recorded microphone samples".to_string())?
            .clone();
        Ok(RecordedAudio {
            samples,
            sample_rate: self.sample_rate,
        })
    }
}

fn record_until_stop(
    max_recording: Duration,
    configured_device: Option<&str>,
    control_rx: mpsc::Receiver<SceneSttControl>,
) -> Result<Option<RecordedAudio>, String> {
    let recording = start_recording(configured_device)?;

    let started = Instant::now();
    loop {
        match control_rx.recv_timeout(Duration::from_millis(25)) {
            Ok(SceneSttControl::Stop) => break,
            Ok(SceneSttControl::Cancel) => return Ok(None),
            Err(mpsc::RecvTimeoutError::Timeout) if started.elapsed() >= max_recording => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
        }
    }
    recording.finish().map(Some)
}

fn record_for_duration(
    duration: Duration,
    configured_device: Option<&str>,
) -> Result<RecordedAudio, String> {
    let recording = start_recording(configured_device)?;
    thread::sleep(duration);
    recording.finish()
}

fn start_recording(configured_device: Option<&str>) -> Result<ActiveRecording, String> {
    let host = cpal::default_host();
    let device = select_input_device(&host, configured_device)?;
    let supported = device
        .default_input_config()
        .map_err(|err| format!("failed to read default microphone config: {err}"))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels().max(1) as usize;
    let stream_config = supported.config();
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let stream = build_recording_stream(
        &device,
        &stream_config,
        supported.sample_format(),
        channels,
        samples.clone(),
    )?;
    stream
        .play()
        .map_err(|err| format!("failed to start microphone stream: {err}"))?;
    Ok(ActiveRecording {
        stream,
        samples,
        sample_rate,
    })
}

fn select_input_device(
    host: &cpal::Host,
    configured_device: Option<&str>,
) -> Result<cpal::Device, String> {
    let Some(configured_device) = configured_device
        .map(str::trim)
        .filter(|device| !device.is_empty())
    else {
        return host
            .default_input_device()
            .ok_or_else(|| "no default microphone input device".to_string());
    };

    let devices = host
        .input_devices()
        .map_err(|err| format!("failed to list microphone input devices: {err}"))?;
    let mut available = Vec::new();
    for device in devices {
        let name = device.name().unwrap_or_else(|_| "<unnamed>".to_string());
        if microphone_device_name_matches(&name, configured_device) {
            return Ok(device);
        }
        available.push(name);
    }

    let available = if available.is_empty() {
        "none".to_string()
    } else {
        available.join(", ")
    };
    Err(format!(
        "configured microphone `{configured_device}` was not found; available: {available}"
    ))
}

fn microphone_device_name_matches(actual: &str, requested: &str) -> bool {
    actual.trim() == requested.trim() || actual.trim().eq_ignore_ascii_case(requested.trim())
}

fn build_recording_stream(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, String> {
    let err_fn = |err| log::warn!("Scene STT microphone stream error: {err}");
    match sample_format {
        cpal::SampleFormat::F32 => {
            build_recording_stream_for::<f32>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::F64 => {
            build_recording_stream_for::<f64>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::I8 => {
            build_recording_stream_for::<i8>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::I16 => {
            build_recording_stream_for::<i16>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::I24 => build_recording_stream_for::<cpal::I24>(
            device,
            stream_config,
            channels,
            samples,
            err_fn,
        ),
        cpal::SampleFormat::I32 => {
            build_recording_stream_for::<i32>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::I64 => {
            build_recording_stream_for::<i64>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::U8 => {
            build_recording_stream_for::<u8>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::U16 => {
            build_recording_stream_for::<u16>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::U32 => {
            build_recording_stream_for::<u32>(device, stream_config, channels, samples, err_fn)
        }
        cpal::SampleFormat::U64 => {
            build_recording_stream_for::<u64>(device, stream_config, channels, samples, err_fn)
        }
        format => Err(format!("unsupported microphone sample format: {format}")),
    }
}

fn build_recording_stream_for<T>(
    device: &cpal::Device,
    stream_config: &cpal::StreamConfig,
    channels: usize,
    samples: Arc<Mutex<Vec<f32>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    device
        .build_input_stream(
            stream_config,
            move |data: &[T], _| append_recorded_samples(data, channels, &samples),
            err_fn,
            None,
        )
        .map_err(|err| format!("failed to build microphone stream: {err}"))
}

fn append_recorded_samples<T>(data: &[T], channels: usize, samples: &Arc<Mutex<Vec<f32>>>)
where
    T: cpal::Sample,
    f32: cpal::FromSample<T>,
{
    if let Ok(mut out) = samples.lock() {
        for frame in data.chunks(channels.max(1)) {
            let sum: f32 = frame.iter().map(|sample| f32::from_sample(*sample)).sum();
            out.push(sum / frame.len().max(1) as f32);
        }
    }
}

fn transcribe_whisper_samples(
    model_path: &Path,
    language: &str,
    samples: &[f32],
) -> Result<String, String> {
    let context = cached_whisper_context(model_path)?;
    let mut state = context
        .create_state()
        .map_err(|err| format!("failed to create Whisper state: {err}"))?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    params.set_translate(false);
    params.set_no_context(true);
    params.set_no_timestamps(true);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    state
        .full(params, samples)
        .map_err(|err| format!("Whisper transcription failed: {err}"))?;
    let mut transcript = String::new();
    for segment in state.as_iter() {
        if let Ok(text) = segment.to_str_lossy() {
            transcript.push_str(text.trim());
            transcript.push(' ');
        }
    }
    Ok(transcript)
}

fn cached_whisper_context(model_path: &Path) -> Result<Arc<WhisperContext>, String> {
    let model_path = model_path.to_path_buf();
    let mut cache = WHISPER_CONTEXT_CACHE
        .lock()
        .map_err(|_| "failed to lock Whisper model cache".to_string())?;
    if let Some((cached_path, context)) = cache.as_ref() {
        if cached_path == &model_path {
            return Ok(context.clone());
        }
    }

    let context = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
        .map_err(|err| {
            format!(
                "failed to load Whisper model {}: {err}",
                model_path.display()
            )
        })?;
    let context = Arc::new(context);
    *cache = Some((model_path, context.clone()));
    Ok(context)
}

fn resample_linear_mono(samples: &[f32], source_rate: u32, target_rate: u32) -> Vec<f32> {
    if samples.is_empty() || source_rate == 0 || target_rate == 0 {
        return Vec::new();
    }
    if source_rate == target_rate {
        return samples.to_vec();
    }

    let target_len =
        ((samples.len() as u64 * target_rate as u64) / source_rate as u64).max(1) as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    let mut out = Vec::with_capacity(target_len);
    for idx in 0..target_len {
        let src_pos = idx as f64 * ratio;
        let left = src_pos.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let frac = (src_pos - left as f64) as f32;
        out.push(samples[left] * (1.0 - frac) + samples[right] * frac);
    }
    out
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

    #[test]
    fn visual_stt_whisper_backend_reports_missing_model() {
        let missing_model = std::env::temp_dir().join("gameterm-missing-whisper-model.bin");
        let (_control_tx, control_rx) = mpsc::channel();
        let result = run_stt_backend(
            SceneSttConfig {
                backend: SceneSttBackend::Whisper(SceneWhisperConfig {
                    model_path: missing_model.clone(),
                    language: "en".to_string(),
                    max_recording: Duration::from_secs(1),
                    input_device: None,
                }),
                timeout: Duration::from_secs(2),
                auto_submit: false,
            },
            control_rx,
        );

        assert_eq!(result.status, "Voice failed");
        assert!(result
            .error
            .as_deref()
            .is_some_and(|err| err.contains(&missing_model.display().to_string())));
    }

    #[test]
    fn visual_stt_resamples_mono_to_whisper_rate() {
        let samples = vec![0.0, 1.0, 0.0, -1.0];
        let resampled = resample_linear_mono(&samples, 8_000, WHISPER_SAMPLE_RATE);

        assert_eq!(resampled.len(), samples.len() * 2);
        assert_eq!(resampled[0], 0.0);
        assert!(resampled[1] > 0.0);
    }

    #[test]
    fn visual_stt_microphone_name_matching_accepts_exact_and_case_insensitive() {
        assert!(microphone_device_name_matches(
            "MacBook Pro Microphone",
            "MacBook Pro Microphone"
        ));
        assert!(microphone_device_name_matches(
            "MacBook Pro Microphone",
            "macbook pro microphone"
        ));
        assert!(!microphone_device_name_matches(
            "External Microphone",
            "MacBook Pro Microphone"
        ));
    }

    #[test]
    fn visual_stt_diagnostics_include_configured_microphone() {
        let config = SceneSttConfig {
            backend: SceneSttBackend::Whisper(SceneWhisperConfig {
                model_path: PathBuf::from("/tmp/model.bin"),
                language: "en".to_string(),
                max_recording: Duration::from_secs(8),
                input_device: Some("Studio Mic".to_string()),
            }),
            timeout: Duration::from_secs(2),
            auto_submit: true,
        };
        let lines = config.diagnostics_lines().join("\n");

        assert!(lines.contains("Backend: whisper"));
        assert!(lines.contains("Auto submit: on"));
        assert!(lines.contains("Mic: Studio Mic"));
        assert!(lines.contains("Model: /tmp/model.bin"));
        assert!(lines.contains("Max recording: 8s"));
    }
}
