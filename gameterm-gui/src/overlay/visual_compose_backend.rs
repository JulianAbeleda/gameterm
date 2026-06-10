use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;

const COMPOSE_BACKEND_ENV: &str = "GAMETERM_SCENE_COMPOSE_BACKEND";
const COMPOSE_BACKEND_KIND_ENV: &str = "GAMETERM_SCENE_COMPOSE_BACKEND_KIND";
const COMPOSE_CONFIG_FILE_ENV: &str = "GAMETERM_SCENE_COMPOSE_CONFIG";
const COMPOSE_CODEX_BIN_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_BIN";
const COMPOSE_CODEX_WORKSPACE_ENV: &str = "GAMETERM_SCENE_COMPOSE_WORKSPACE";
const COMPOSE_CODEX_SANDBOX_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_SANDBOX";
const COMPOSE_CODEX_APPROVAL_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_APPROVAL";
const COMPOSE_CODEX_TIMEOUT_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_TIMEOUT_SECONDS";
const COMPOSE_CODEX_REASONING_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_REASONING";
const COMPOSE_CODEX_MODEL_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_MODEL";
const COMPOSE_CONFIG_FILE_NAME: &str = "scene-compose.json";
const DEFAULT_CODEX_APPROVAL_POLICY: &str = "on-request";
const COMPOSE_BACKEND_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_CODEX_TIMEOUT: Duration = Duration::from_secs(90);
const COMPOSE_BACKEND_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ComposeBackendRequest {
    pub(super) prompt: String,
    pub(super) backend_prompt: String,
    pub(super) scene_path: Option<String>,
    pub(super) pane_id: Option<usize>,
    /// Session-local model override from `/model`. Wins over the configured
    /// model when set; otherwise the configured/global model is used.
    pub(super) model_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ComposeBackendResult {
    pub(super) prompt: String,
    pub(super) stdout: String,
    pub(super) stderr: String,
    pub(super) exit_code: Option<i32>,
    pub(super) label: ComposeBackendLabel,
}

impl ComposeBackendResult {
    pub(super) fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub(super) fn failure_status(&self) -> String {
        match classify_compose_failure(self) {
            ComposeFailureKind::CodexRateLimited | ComposeFailureKind::CodexAuthBlocked => {
                "Codex unavailable".to_string()
            }
            ComposeFailureKind::CodexTimedOut | ComposeFailureKind::ComposeTimedOut => {
                format!("{} timed out", self.label.name())
            }
            ComposeFailureKind::Canceled => format!("{} canceled", self.label.name()),
            _ => self.label.failed_status().to_string(),
        }
    }

    pub(super) fn failure_dialogue(&self, sanitized_stderr: &str) -> String {
        match classify_compose_failure(self) {
            ComposeFailureKind::MissingCodexBinary => {
                format!(
                    "Codex is unavailable because the configured binary could not be launched. {sanitized_stderr}"
                )
            }
            ComposeFailureKind::CodexRateLimited => {
                "Codex is currently rate limited. Try again after your Codex limit resets.".to_string()
            }
            ComposeFailureKind::CodexAuthBlocked => {
                "Codex could not connect to the Codex service. Check your Codex auth/session state, then try again.".to_string()
            }
            ComposeFailureKind::CodexTimedOut => {
                format!("Codex timed out before returning a reply for: {}", self.prompt)
            }
            ComposeFailureKind::ComposeTimedOut => {
                format!("Compose timed out before returning a reply for: {}", self.prompt)
            }
            ComposeFailureKind::Canceled => {
                format!("{} canceled for: {}", self.label.name(), self.prompt)
            }
            ComposeFailureKind::EmptyDiagnostic => {
                format!("{} failed for: {}", self.label.name(), self.prompt)
            }
            ComposeFailureKind::Other => sanitized_stderr.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ComposeBackendLabel {
    Compose,
    Codex,
}

impl ComposeBackendLabel {
    fn name(self) -> &'static str {
        match self {
            ComposeBackendLabel::Compose => "Compose",
            ComposeBackendLabel::Codex => "Codex",
        }
    }

    fn running_status(self, prompt: &str) -> String {
        match self {
            ComposeBackendLabel::Compose => format!("Compose running: {prompt}"),
            ComposeBackendLabel::Codex => format!("Codex running: {prompt}"),
        }
    }

    pub(super) fn succeeded_status(self) -> &'static str {
        match self {
            ComposeBackendLabel::Compose => "Compose succeeded",
            ComposeBackendLabel::Codex => "Codex succeeded",
        }
    }

    pub(super) fn failed_status(self) -> &'static str {
        match self {
            ComposeBackendLabel::Compose => "Compose failed",
            ComposeBackendLabel::Codex => "Codex failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComposeFailureKind {
    MissingCodexBinary,
    CodexRateLimited,
    CodexAuthBlocked,
    CodexTimedOut,
    ComposeTimedOut,
    Canceled,
    EmptyDiagnostic,
    Other,
}

fn classify_compose_failure(result: &ComposeBackendResult) -> ComposeFailureKind {
    let diagnostic = result.stderr.trim();
    if diagnostic.is_empty() {
        return ComposeFailureKind::EmptyDiagnostic;
    }
    if result.label == ComposeBackendLabel::Codex {
        if diagnostic.contains("failed to spawn Codex compose backend") {
            return ComposeFailureKind::MissingCodexBinary;
        }
        if diagnostic.contains("429 Too Many Requests")
            || diagnostic.contains("exceeded retry limit")
        {
            return ComposeFailureKind::CodexRateLimited;
        }
        if diagnostic.contains("403 Forbidden")
            || diagnostic.contains("failed to connect to websocket")
        {
            return ComposeFailureKind::CodexAuthBlocked;
        }
        if diagnostic.contains("Codex compose backend timed out") {
            return ComposeFailureKind::CodexTimedOut;
        }
    }
    if diagnostic.contains("compose backend timed out") {
        return ComposeFailureKind::ComposeTimedOut;
    }
    if diagnostic.contains("compose backend canceled") {
        return ComposeFailureKind::Canceled;
    }
    ComposeFailureKind::Other
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ComposeBackendConfig {
    BuiltIn,
    Command(String),
    Codex(CodexComposeConfig),
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexComposeConfig {
    pub(super) program: String,
    pub(super) workspace: PathBuf,
    pub(super) sandbox: String,
    pub(super) approval: String,
    pub(super) reasoning_effort: Option<String>,
    pub(super) model: Option<String>,
    pub(super) json: bool,
    pub(super) timeout: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct SceneComposeConfigFile {
    #[serde(default)]
    backend_kind: Option<String>,
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    codex_bin: Option<String>,
    #[serde(default)]
    codex_workspace: Option<PathBuf>,
    #[serde(default)]
    codex_sandbox: Option<String>,
    #[serde(default)]
    codex_approval: Option<String>,
    #[serde(default)]
    codex_timeout_seconds: Option<u64>,
    #[serde(default)]
    codex_reasoning_effort: Option<String>,
    #[serde(default)]
    codex_model: Option<String>,
}

pub(super) fn compose_running_status(prompt: &str) -> String {
    match compose_backend_config_from_env() {
        ComposeBackendConfig::Codex(_) => ComposeBackendLabel::Codex.running_status(prompt),
        ComposeBackendConfig::BuiltIn
        | ComposeBackendConfig::Command(_)
        | ComposeBackendConfig::Invalid(_) => ComposeBackendLabel::Compose.running_status(prompt),
    }
}

/// Cancels an in-flight compose backend request. Cancellation kills the
/// backend child process; the worker still delivers a result through the
/// normal channel so the overlay keeps a single completion path.
#[derive(Debug, Clone)]
pub(super) struct ComposeBackendCancel(Arc<AtomicBool>);

impl ComposeBackendCancel {
    pub(super) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub(super) fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn is_canceled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub(super) fn spawn_compose_backend(
    request: ComposeBackendRequest,
    tx: mpsc::Sender<ComposeBackendResult>,
) -> ComposeBackendCancel {
    let cancel = ComposeBackendCancel::new();
    let worker_cancel = cancel.clone();
    thread::spawn(move || {
        let result = run_compose_backend(request, &worker_cancel);
        let _ = tx.send(result);
    });
    cancel
}

fn run_compose_backend(
    request: ComposeBackendRequest,
    cancel: &ComposeBackendCancel,
) -> ComposeBackendResult {
    match compose_backend_config_from_env() {
        ComposeBackendConfig::BuiltIn => ComposeBackendResult {
            stdout: deterministic_compose_reply(&request.prompt),
            stderr: String::new(),
            exit_code: Some(0),
            prompt: request.prompt,
            label: ComposeBackendLabel::Compose,
        },
        ComposeBackendConfig::Command(command) => {
            run_configured_compose_backend(request, command, cancel)
        }
        ComposeBackendConfig::Codex(config) => run_codex_compose_backend(request, config, cancel),
        ComposeBackendConfig::Invalid(err) => ComposeBackendResult {
            stdout: String::new(),
            stderr: err,
            exit_code: None,
            prompt: request.prompt,
            label: ComposeBackendLabel::Compose,
        },
    }
}

pub(super) fn compose_backend_config_from_env() -> ComposeBackendConfig {
    let file_config = match scene_compose_config_from_file() {
        Ok(config) => config,
        Err(err) => return ComposeBackendConfig::Invalid(err),
    };
    compose_backend_config_from_sources(&file_config, &SceneComposeEnv::current())
}

#[cfg(test)]
pub(super) fn compose_backend_config(
    kind: Option<&str>,
    backend: Option<&str>,
    codex_config: CodexComposeConfig,
) -> ComposeBackendConfig {
    compose_backend_config_with_codex_loader(kind, backend, || Ok(codex_config))
}

fn compose_backend_config_with_codex_loader(
    kind: Option<&str>,
    backend: Option<&str>,
    load_codex_config: impl FnOnce() -> Result<CodexComposeConfig, String>,
) -> ComposeBackendConfig {
    if let Some(kind) = kind.map(str::trim).filter(|value| !value.is_empty()) {
        if kind.eq_ignore_ascii_case("codex") {
            return codex_backend_config(load_codex_config);
        }
        if kind.eq_ignore_ascii_case("built_in")
            || kind.eq_ignore_ascii_case("builtin")
            || kind.eq_ignore_ascii_case("deterministic")
        {
            return ComposeBackendConfig::BuiltIn;
        }
        if kind.eq_ignore_ascii_case("command") {
            return match backend.map(str::trim).filter(|value| !value.is_empty()) {
                Some(value) => ComposeBackendConfig::Command(value.to_string()),
                None => ComposeBackendConfig::Invalid(
                    "Scene compose backend_kind command requires backend or command".to_string(),
                ),
            };
        }
        return ComposeBackendConfig::Invalid(format!(
            "invalid Scene compose backend_kind `{kind}`; expected built_in, command, or codex"
        ));
    }
    match backend.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.eq_ignore_ascii_case("codex") => {
            codex_backend_config(load_codex_config)
        }
        Some(value) => ComposeBackendConfig::Command(value.to_string()),
        None => ComposeBackendConfig::BuiltIn,
    }
}

fn codex_backend_config(
    load_codex_config: impl FnOnce() -> Result<CodexComposeConfig, String>,
) -> ComposeBackendConfig {
    match load_codex_config() {
        Ok(config) => ComposeBackendConfig::Codex(config),
        Err(err) => ComposeBackendConfig::Invalid(err),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SceneComposeEnv {
    backend_kind: Option<String>,
    backend: Option<String>,
    config_file: Option<PathBuf>,
    codex_bin: Option<String>,
    codex_workspace: Option<PathBuf>,
    codex_sandbox: Option<String>,
    codex_approval: Option<String>,
    codex_timeout_seconds: Option<String>,
    codex_reasoning_effort: Option<String>,
    codex_model: Option<String>,
}

impl SceneComposeEnv {
    fn current() -> Self {
        Self {
            backend_kind: non_empty_env(COMPOSE_BACKEND_KIND_ENV),
            backend: non_empty_env(COMPOSE_BACKEND_ENV),
            config_file: non_empty_env(COMPOSE_CONFIG_FILE_ENV).map(PathBuf::from),
            codex_bin: non_empty_env(COMPOSE_CODEX_BIN_ENV),
            codex_workspace: non_empty_env(COMPOSE_CODEX_WORKSPACE_ENV).map(PathBuf::from),
            codex_sandbox: non_empty_env(COMPOSE_CODEX_SANDBOX_ENV),
            codex_approval: non_empty_env(COMPOSE_CODEX_APPROVAL_ENV),
            codex_timeout_seconds: non_empty_env(COMPOSE_CODEX_TIMEOUT_ENV),
            codex_reasoning_effort: non_empty_env(COMPOSE_CODEX_REASONING_ENV),
            codex_model: non_empty_env(COMPOSE_CODEX_MODEL_ENV),
        }
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn scene_compose_config_from_file() -> Result<SceneComposeConfigFile, String> {
    let path = scene_compose_config_path(&SceneComposeEnv::current());
    if !path.exists() {
        return Ok(SceneComposeConfigFile::default());
    }
    let data = std::fs::read_to_string(&path).map_err(|err| {
        format!(
            "failed to read Scene compose config {}: {err}",
            path.display()
        )
    })?;
    serde_json::from_str::<SceneComposeConfigFile>(&data).map_err(|err| {
        format!(
            "failed to parse Scene compose config {}: {err}",
            path.display()
        )
    })
}

fn scene_compose_config_path(env: &SceneComposeEnv) -> PathBuf {
    env.config_file.clone().unwrap_or_else(|| {
        config::CONFIG_DIRS
            .first()
            .cloned()
            .unwrap_or_else(|| config::HOME_DIR.join(".config").join("gameterm"))
            .join(COMPOSE_CONFIG_FILE_NAME)
    })
}

fn compose_backend_config_from_sources(
    file_config: &SceneComposeConfigFile,
    env: &SceneComposeEnv,
) -> ComposeBackendConfig {
    let backend_kind = env
        .backend_kind
        .as_deref()
        .or(file_config.backend_kind.as_deref());
    let backend = env
        .backend
        .as_deref()
        .or(file_config.backend.as_deref())
        .or(file_config.command.as_deref());
    compose_backend_config_with_codex_loader(backend_kind, backend, || {
        codex_compose_config_from_sources(file_config, env)
    })
}

fn codex_compose_config_from_sources(
    file_config: &SceneComposeConfigFile,
    env: &SceneComposeEnv,
) -> Result<CodexComposeConfig, String> {
    let sandbox = env
        .codex_sandbox
        .as_deref()
        .or(file_config.codex_sandbox.as_deref())
        .unwrap_or("read-only");
    let approval = env
        .codex_approval
        .as_deref()
        .or(file_config.codex_approval.as_deref())
        .unwrap_or(DEFAULT_CODEX_APPROVAL_POLICY);
    let reasoning_effort = env
        .codex_reasoning_effort
        .as_deref()
        .or(file_config.codex_reasoning_effort.as_deref())
        .map(validate_codex_reasoning_effort)
        .transpose()?;
    let model = env
        .codex_model
        .clone()
        .or_else(|| file_config.codex_model.clone());
    Ok(CodexComposeConfig {
        program: env
            .codex_bin
            .clone()
            .or_else(|| file_config.codex_bin.clone())
            .unwrap_or_else(|| "codex".to_string()),
        workspace: env
            .codex_workspace
            .clone()
            .or_else(|| file_config.codex_workspace.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from(".")),
        sandbox: validate_codex_sandbox(sandbox)?,
        approval: validate_codex_approval(approval)?,
        reasoning_effort,
        model,
        json: true,
        timeout: codex_timeout_from_sources(file_config, env)?,
    })
}

fn validate_codex_sandbox(value: &str) -> Result<String, String> {
    match value.trim() {
        "read-only" | "workspace-write" | "danger-full-access" => Ok(value.trim().to_string()),
        other => Err(format!(
            "invalid Scene Codex sandbox `{other}`; expected read-only, workspace-write, or danger-full-access"
        )),
    }
}

fn validate_codex_reasoning_effort(value: &str) -> Result<String, String> {
    match value.trim() {
        "minimal" | "low" | "medium" | "high" | "xhigh" => Ok(value.trim().to_string()),
        other => Err(format!(
            "invalid Scene Codex reasoning effort `{other}`; expected minimal, low, medium, high, or xhigh"
        )),
    }
}

fn validate_codex_approval(value: &str) -> Result<String, String> {
    match value.trim() {
        "on-request" | "never" | "untrusted" => Ok(value.trim().to_string()),
        other => Err(format!(
            "invalid Scene Codex approval `{other}`; expected on-request, never, or untrusted"
        )),
    }
}

fn codex_timeout_from_sources(
    file_config: &SceneComposeConfigFile,
    env: &SceneComposeEnv,
) -> Result<Duration, String> {
    let seconds = match env.codex_timeout_seconds.as_deref() {
        Some(value) => value
            .parse::<u64>()
            .map_err(|err| format!("invalid {COMPOSE_CODEX_TIMEOUT_ENV} value `{value}`: {err}"))?,
        None => file_config
            .codex_timeout_seconds
            .unwrap_or(DEFAULT_CODEX_TIMEOUT.as_secs()),
    };
    if seconds == 0 {
        return Err("Scene Codex timeout must be greater than 0 seconds".to_string());
    }
    Ok(Duration::from_secs(seconds))
}

pub(super) fn run_configured_compose_backend(
    request: ComposeBackendRequest,
    command: String,
    cancel: &ComposeBackendCancel,
) -> ComposeBackendResult {
    let argv = match parse_compose_backend_argv(&command) {
        Ok(argv) => argv,
        Err(err) => {
            return ComposeBackendResult {
                prompt: request.prompt,
                stdout: String::new(),
                stderr: err,
                exit_code: None,
                label: ComposeBackendLabel::Compose,
            };
        }
    };
    let Some((program, args)) = argv.split_first() else {
        return ComposeBackendResult {
            prompt: request.prompt,
            stdout: String::new(),
            stderr: "empty compose backend command".to_string(),
            exit_code: None,
            label: ComposeBackendLabel::Compose,
        };
    };

    let mut child = match backend_command(program, args, &request)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return ComposeBackendResult {
                prompt: request.prompt,
                stdout: String::new(),
                stderr: format!("failed to spawn compose backend: {err}"),
                exit_code: None,
                label: ComposeBackendLabel::Compose,
            };
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request.backend_prompt.as_bytes());
    }

    match wait_for_child_output(child, COMPOSE_BACKEND_TIMEOUT, cancel) {
        Ok(output) => ComposeBackendResult {
            prompt: request.prompt,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.exit_code,
            label: ComposeBackendLabel::Compose,
        },
        Err(err) => ComposeBackendResult {
            prompt: request.prompt,
            stdout: String::from_utf8_lossy(&err.stdout).to_string(),
            stderr: err.stderr,
            exit_code: None,
            label: ComposeBackendLabel::Compose,
        },
    }
}

fn parse_compose_backend_argv(command: &str) -> Result<Vec<String>, String> {
    shlex::split(command).ok_or_else(|| "invalid compose backend command quoting".to_string())
}

pub(super) fn run_codex_compose_backend(
    request: ComposeBackendRequest,
    mut config: CodexComposeConfig,
    cancel: &ComposeBackendCancel,
) -> ComposeBackendResult {
    if request.model_override.is_some() {
        config.model = request.model_override.clone();
    }
    let output_file = std::env::temp_dir().join(format!(
        "gameterm-scene-codex-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let argv = codex_compose_argv(&config, &output_file, &request.backend_prompt);
    let Some((program, args)) = argv.split_first() else {
        return ComposeBackendResult {
            prompt: request.prompt,
            stdout: String::new(),
            stderr: "empty Codex backend command".to_string(),
            exit_code: None,
            label: ComposeBackendLabel::Codex,
        };
    };

    let result = run_codex_command(
        request.clone(),
        program,
        args,
        &output_file,
        config.timeout,
        cancel,
    );
    let _ = std::fs::remove_file(output_file);
    result
}

pub(super) fn codex_compose_argv(
    config: &CodexComposeConfig,
    output_file: &Path,
    prompt: &str,
) -> Vec<String> {
    let mut argv = vec![
        config.program.clone(),
        "exec".to_string(),
        "--output-last-message".to_string(),
        output_file.display().to_string(),
        "-C".to_string(),
        config.workspace.display().to_string(),
        "-s".to_string(),
        config.sandbox.clone(),
        "-c".to_string(),
        format!("approval_policy=\"{}\"", config.approval),
    ];
    if let Some(effort) = config.reasoning_effort.as_deref() {
        argv.push("-c".to_string());
        argv.push(format!("model_reasoning_effort=\"{effort}\""));
    }
    if let Some(model) = config.model.as_deref() {
        argv.push("-m".to_string());
        argv.push(model.to_string());
    }
    if config.json {
        argv.push("--json".to_string());
    }
    argv.push(prompt.to_string());
    argv
}

fn run_codex_command(
    request: ComposeBackendRequest,
    program: &str,
    args: &[String],
    output_file: &Path,
    timeout: Duration,
    cancel: &ComposeBackendCancel,
) -> ComposeBackendResult {
    let child = match backend_command(program, args, &request)
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            return ComposeBackendResult {
                prompt: request.prompt,
                stdout: String::new(),
                stderr: format!("failed to spawn Codex compose backend `{program}`: {err}"),
                exit_code: None,
                label: ComposeBackendLabel::Codex,
            };
        }
    };

    match wait_for_child_output(child, timeout, cancel) {
        Ok(output) => ComposeBackendResult {
            prompt: request.prompt,
            stdout: codex_output_text(output_file, &output.stdout),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.exit_code,
            label: ComposeBackendLabel::Codex,
        },
        Err(err) => ComposeBackendResult {
            prompt: request.prompt,
            stdout: codex_output_text(output_file, &err.stdout),
            stderr: err
                .stderr
                .replace("compose backend", "Codex compose backend"),
            exit_code: None,
            label: ComposeBackendLabel::Codex,
        },
    }
}

fn backend_command(program: &str, args: &[String], request: &ComposeBackendRequest) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .env("GAMETERM_SCENE_COMPOSE_PROMPT", &request.prompt)
        .env("GAMETERM_SCENE_COMPOSE_CONTEXT", &request.backend_prompt)
        .env(
            "GAMETERM_SCENE_COMPOSE_SESSION_ID",
            request
                .pane_id
                .map(|pane_id| format!("pane-{pane_id}"))
                .unwrap_or_else(|| "scene".to_string()),
        )
        .env(
            "GAMETERM_SCENE_PATH",
            request.scene_path.as_deref().unwrap_or(""),
        )
        .env(
            "GAMETERM_SCENE_PANE_ID",
            request
                .pane_id
                .map(|pane_id| pane_id.to_string())
                .unwrap_or_default(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[derive(Debug)]
struct CollectedProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: Option<i32>,
}

struct CollectedProcessError {
    stdout: Vec<u8>,
    stderr: String,
}

fn wait_for_child_output(
    mut child: Child,
    timeout: Duration,
    cancel: &ComposeBackendCancel,
) -> Result<CollectedProcessOutput, CollectedProcessError> {
    let stdout = child.stdout.take().map(read_pipe);
    let stderr = child.stderr.take().map(read_pipe);
    let deadline = Instant::now() + timeout;

    loop {
        if cancel.is_canceled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CollectedProcessError {
                stdout: join_pipe(stdout),
                stderr: "compose backend canceled".to_string(),
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(CollectedProcessOutput {
                    stdout: join_pipe(stdout),
                    stderr: join_pipe(stderr),
                    exit_code: status.code(),
                });
            }
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(COMPOSE_BACKEND_POLL_INTERVAL);
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CollectedProcessError {
                    stdout: join_pipe(stdout),
                    stderr: format!("compose backend timed out after {}s", timeout.as_secs()),
                });
            }
            Err(err) => {
                return Err(CollectedProcessError {
                    stdout: join_pipe(stdout),
                    stderr: format!("failed to wait for compose backend: {err}"),
                });
            }
        }
    }
}

fn read_pipe<R>(mut pipe: R) -> thread::JoinHandle<Vec<u8>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut output = Vec::new();
        let _ = pipe.read_to_end(&mut output);
        output
    })
}

fn join_pipe(handle: Option<thread::JoinHandle<Vec<u8>>>) -> Vec<u8> {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

pub(super) fn codex_output_text(output_file: &Path, stdout: &[u8]) -> String {
    std::fs::read_to_string(output_file)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| String::from_utf8_lossy(stdout).to_string())
}

fn deterministic_compose_reply(prompt: &str) -> String {
    format!("Built-in compose reply: {}", prompt.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(prompt: &str) -> ComposeBackendRequest {
        ComposeBackendRequest {
            prompt: prompt.to_string(),
            backend_prompt: prompt.to_string(),
            scene_path: Some("scene.json".to_string()),
            pane_id: Some(7),
            model_override: None,
        }
    }

    fn executable_script(path: &Path, body: &str) {
        std::fs::write(path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).unwrap();
        }
    }

    #[test]
    fn codex_compose_argv_includes_reasoning_effort_when_configured() {
        let mut config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace"),
            sandbox: "read-only".to_string(),
            approval: "never".to_string(),
            reasoning_effort: Some("low".to_string()),
            model: Some("gpt-5.3-codex-spark".to_string()),
            json: true,
            timeout: DEFAULT_CODEX_TIMEOUT,
        };
        let argv = codex_compose_argv(&config, Path::new("/tmp/out.txt"), "hi");
        let joined = argv.join(" ");
        assert!(joined.contains("model_reasoning_effort=\"low\""));
        assert!(joined.contains("-m gpt-5.3-codex-spark"));

        config.reasoning_effort = None;
        let argv = codex_compose_argv(&config, Path::new("/tmp/out.txt"), "hi");
        assert!(!argv.join(" ").contains("model_reasoning_effort"));
    }

    #[test]
    fn codex_reasoning_effort_rejects_unknown_values() {
        assert!(validate_codex_reasoning_effort("low").is_ok());
        assert!(validate_codex_reasoning_effort("xhigh").is_ok());
        assert!(validate_codex_reasoning_effort("turbo").is_err());
    }

    #[test]
    fn wait_for_child_output_cancel_kills_backend() {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", "sleep 30"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command.spawn().unwrap();
        let cancel = ComposeBackendCancel::new();
        cancel.cancel();
        let started = Instant::now();

        let err = wait_for_child_output(child, Duration::from_secs(30), &cancel).unwrap_err();

        assert!(err.stderr.contains("compose backend canceled"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn canceled_result_reports_canceled_status_and_dialogue() {
        let result = ComposeBackendResult {
            prompt: "slow question".to_string(),
            stdout: String::new(),
            stderr: "Codex compose backend canceled".to_string(),
            exit_code: None,
            label: ComposeBackendLabel::Codex,
        };

        assert_eq!(result.failure_status(), "Codex canceled");
        assert_eq!(
            result.failure_dialogue("Codex compose backend canceled"),
            "Codex canceled for: slow question"
        );
    }

    #[test]
    fn compose_backend_config_selects_codex_explicitly() {
        let codex_config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace"),
            sandbox: "read-only".to_string(),
            approval: DEFAULT_CODEX_APPROVAL_POLICY.to_string(),
            reasoning_effort: None,
            model: None,
            json: true,
            timeout: DEFAULT_CODEX_TIMEOUT,
        };

        assert_eq!(
            compose_backend_config(None, None, codex_config.clone()),
            ComposeBackendConfig::BuiltIn
        );
        assert_eq!(
            compose_backend_config(None, Some("helper --flag"), codex_config.clone()),
            ComposeBackendConfig::Command("helper --flag".to_string())
        );
        assert_eq!(
            compose_backend_config(Some("codex"), None, codex_config.clone()),
            ComposeBackendConfig::Codex(codex_config.clone())
        );
        assert_eq!(
            compose_backend_config(None, Some("codex"), codex_config.clone()),
            ComposeBackendConfig::Codex(codex_config)
        );
    }

    #[test]
    fn compose_backend_config_honors_explicit_backend_kind() {
        let codex_config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace"),
            sandbox: "read-only".to_string(),
            approval: DEFAULT_CODEX_APPROVAL_POLICY.to_string(),
            reasoning_effort: None,
            model: None,
            json: true,
            timeout: DEFAULT_CODEX_TIMEOUT,
        };

        assert_eq!(
            compose_backend_config(
                Some("built_in"),
                Some("helper --flag"),
                codex_config.clone()
            ),
            ComposeBackendConfig::BuiltIn
        );
        assert_eq!(
            compose_backend_config(Some("command"), Some("helper --flag"), codex_config.clone()),
            ComposeBackendConfig::Command("helper --flag".to_string())
        );
        assert!(matches!(
            compose_backend_config(Some("unknown"), None, codex_config),
            ComposeBackendConfig::Invalid(err) if err.contains("invalid Scene compose backend_kind")
        ));
    }

    #[test]
    fn compose_backend_argv_supports_quoted_paths_and_args() {
        assert_eq!(
            parse_compose_backend_argv(r#"/tmp/my\ helper --mode "short reply""#).unwrap(),
            vec!["/tmp/my helper", "--mode", "short reply"]
        );
    }

    #[test]
    fn compose_backend_argv_rejects_invalid_quoting() {
        assert_eq!(
            parse_compose_backend_argv(r#""unterminated"#),
            Err("invalid compose backend command quoting".to_string())
        );
    }

    #[test]
    fn run_configured_compose_backend_handles_paths_with_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fake compose.sh");
        executable_script(
            &script,
            "#!/usr/bin/env sh\nprintf 'reply:%s:%s:%s\\n' \"$1\" \"$GAMETERM_SCENE_COMPOSE_PROMPT\" \"$GAMETERM_SCENE_PANE_ID\"\n",
        );

        let result = run_configured_compose_backend(
            request("hello"),
            format!(
                "{} \"short reply\"",
                shlex::try_quote(&script.display().to_string()).unwrap()
            ),
            &ComposeBackendCancel::new(),
        );

        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "reply:short reply:hello:7\n");
    }

    #[test]
    fn run_configured_compose_backend_receives_context_prompt_on_stdin() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("stdin-compose.sh");
        executable_script(
            &script,
            "#!/usr/bin/env sh\nprintf 'env:%s\\n' \"$GAMETERM_SCENE_COMPOSE_PROMPT\"\nprintf 'context:%s\\n' \"$GAMETERM_SCENE_COMPOSE_CONTEXT\"\nprintf 'stdin:'\ncat\n",
        );
        let mut request = request("11249");
        request.backend_prompt = "Latest user prompt:\n11249\n\nRecent turns:\nUser: whats the weather today?\nCodex: What city or ZIP code should I check?".to_string();

        let result = run_configured_compose_backend(
            request,
            script.display().to_string(),
            &ComposeBackendCancel::new(),
        );

        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("env:11249"));
        assert!(result.stdout.contains("context:Latest user prompt:"));
        assert!(result.stdout.contains("stdin:Latest user prompt:"));
        assert!(result
            .stdout
            .contains("What city or ZIP code should I check?"));
    }

    #[test]
    fn run_configured_compose_backend_collects_large_stdout_without_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("large-stdout.sh");
        executable_script(
            &script,
            "#!/usr/bin/env sh\nyes x | head -n 20000\nprintf done\n",
        );

        let result = run_configured_compose_backend(
            request("large"),
            script.display().to_string(),
            &ComposeBackendCancel::new(),
        );

        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("done"));
        assert!(result.stdout.len() > 20_000);
    }

    #[test]
    fn run_configured_compose_backend_collects_large_stderr_without_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("large-stderr.sh");
        executable_script(
            &script,
            "#!/usr/bin/env sh\nyes err | head -n 20000 >&2\nexit 3\n",
        );

        let result = run_configured_compose_backend(
            request("large"),
            script.display().to_string(),
            &ComposeBackendCancel::new(),
        );

        assert_eq!(result.exit_code, Some(3));
        assert!(result.stderr.len() > 20_000);
    }

    #[test]
    fn wait_for_child_output_kills_timed_out_backend() {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("printf started; sleep 5; printf never")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take();

        // The timeout must outlive shell startup under parallel test load so
        // the child reliably prints `started` before it is killed.
        let err = wait_for_child_output(
            child,
            Duration::from_millis(400),
            &ComposeBackendCancel::new(),
        )
        .unwrap_err();

        assert_eq!(String::from_utf8_lossy(&err.stdout), "started");
        assert_eq!(err.stderr, "compose backend timed out after 0s");
    }

    #[test]
    fn codex_compose_argv_uses_structured_arguments() {
        let config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace with spaces"),
            sandbox: "read-only".to_string(),
            approval: "on-request".to_string(),
            reasoning_effort: None,
            model: None,
            json: true,
            timeout: DEFAULT_CODEX_TIMEOUT,
        };
        let argv = codex_compose_argv(
            &config,
            Path::new("/tmp/last-message.txt"),
            "inspect roadmap && do not shell split",
        );

        assert_eq!(argv[0], "codex");
        assert_eq!(argv[1], "exec");
        assert!(argv.contains(&"--json".to_string()));
        assert_eq!(
            argv,
            vec![
                "codex",
                "exec",
                "--output-last-message",
                "/tmp/last-message.txt",
                "-C",
                "/workspace with spaces",
                "-s",
                "read-only",
                "-c",
                "approval_policy=\"on-request\"",
                "--json",
                "inspect roadmap && do not shell split"
            ]
        );
    }

    #[test]
    fn codex_output_prefers_last_message_file() {
        let dir = tempfile::tempdir().unwrap();
        let output_file = dir.path().join("last.txt");
        std::fs::write(&output_file, "final Codex reply\n").unwrap();

        assert_eq!(
            codex_output_text(&output_file, b"{\"event\":\"stdout fallback\"}\n"),
            "final Codex reply\n"
        );
    }

    #[test]
    fn codex_backend_fake_command_returns_last_message_file() {
        let dir = tempfile::tempdir().unwrap();
        let fake_codex = dir.path().join("fake-codex.sh");
        executable_script(
            &fake_codex,
            "#!/usr/bin/env sh\nwhile [ \"$1\" != \"\" ]; do\n  if [ \"$1\" = \"--output-last-message\" ]; then\n    shift\n    printf 'Codex says: %s\\n' \"$GAMETERM_SCENE_COMPOSE_PROMPT\" > \"$1\"\n  fi\n  shift || exit 0\ndone\nprintf '{\"event\":\"done\"}\\n'\n",
        );

        let config = CodexComposeConfig {
            program: fake_codex.display().to_string(),
            workspace: dir.path().to_path_buf(),
            sandbox: "read-only".to_string(),
            approval: "on-request".to_string(),
            reasoning_effort: None,
            model: None,
            json: true,
            timeout: DEFAULT_CODEX_TIMEOUT,
        };
        let result = run_codex_compose_backend(
            request("look at roadmap"),
            config,
            &ComposeBackendCancel::new(),
        );

        assert_eq!(result.label, ComposeBackendLabel::Codex);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "Codex says: look at roadmap\n");
    }

    #[test]
    fn compose_backend_config_uses_file_config_for_codex() {
        let file_config = SceneComposeConfigFile {
            backend_kind: Some("codex".to_string()),
            codex_bin: Some("/opt/homebrew/bin/codex".to_string()),
            codex_workspace: Some(PathBuf::from("/workspace")),
            codex_sandbox: Some("workspace-write".to_string()),
            codex_approval: Some("never".to_string()),
            codex_timeout_seconds: Some(120),
            ..SceneComposeConfigFile::default()
        };

        let config = compose_backend_config_from_sources(&file_config, &SceneComposeEnv::default());

        assert_eq!(
            config,
            ComposeBackendConfig::Codex(CodexComposeConfig {
                program: "/opt/homebrew/bin/codex".to_string(),
                workspace: PathBuf::from("/workspace"),
                sandbox: "workspace-write".to_string(),
                approval: "never".to_string(),
                reasoning_effort: None,
                model: None,
                json: true,
                timeout: Duration::from_secs(120),
            })
        );
    }

    #[test]
    fn compose_backend_config_env_overrides_file_config() {
        let file_config = SceneComposeConfigFile {
            backend_kind: Some("built_in".to_string()),
            codex_timeout_seconds: Some(45),
            ..SceneComposeConfigFile::default()
        };
        let env = SceneComposeEnv {
            backend_kind: Some("codex".to_string()),
            codex_bin: Some("/tmp/fake-codex".to_string()),
            codex_workspace: Some(PathBuf::from("/env-workspace")),
            codex_sandbox: Some("read-only".to_string()),
            codex_approval: Some("on-request".to_string()),
            codex_timeout_seconds: Some("30".to_string()),
            ..SceneComposeEnv::default()
        };

        let config = compose_backend_config_from_sources(&file_config, &env);

        assert_eq!(
            config,
            ComposeBackendConfig::Codex(CodexComposeConfig {
                program: "/tmp/fake-codex".to_string(),
                workspace: PathBuf::from("/env-workspace"),
                sandbox: "read-only".to_string(),
                approval: "on-request".to_string(),
                reasoning_effort: None,
                model: None,
                json: true,
                timeout: Duration::from_secs(30),
            })
        );
    }

    #[test]
    fn compose_backend_config_rejects_invalid_codex_values() {
        let file_config = SceneComposeConfigFile {
            backend_kind: Some("codex".to_string()),
            codex_sandbox: Some("open".to_string()),
            ..SceneComposeConfigFile::default()
        };

        let config = compose_backend_config_from_sources(&file_config, &SceneComposeEnv::default());

        assert!(
            matches!(config, ComposeBackendConfig::Invalid(err) if err.contains("invalid Scene Codex sandbox"))
        );
    }

    #[test]
    fn compose_backend_config_ignores_unused_invalid_codex_values() {
        let file_config = SceneComposeConfigFile {
            backend_kind: Some("built_in".to_string()),
            backend: Some("helper --flag".to_string()),
            codex_sandbox: Some("open".to_string()),
            codex_approval: Some("ask-me-later".to_string()),
            codex_timeout_seconds: Some(0),
            ..SceneComposeConfigFile::default()
        };

        let config = compose_backend_config_from_sources(&file_config, &SceneComposeEnv::default());

        assert_eq!(config, ComposeBackendConfig::BuiltIn);

        let file_config = SceneComposeConfigFile {
            backend_kind: Some("command".to_string()),
            backend: Some("helper --flag".to_string()),
            codex_sandbox: Some("open".to_string()),
            codex_approval: Some("ask-me-later".to_string()),
            codex_timeout_seconds: Some(0),
            ..SceneComposeConfigFile::default()
        };

        let config = compose_backend_config_from_sources(&file_config, &SceneComposeEnv::default());

        assert_eq!(
            config,
            ComposeBackendConfig::Command("helper --flag".to_string())
        );
    }

    #[test]
    fn codex_timeout_rejects_zero() {
        let file_config = SceneComposeConfigFile {
            codex_timeout_seconds: Some(0),
            ..SceneComposeConfigFile::default()
        };

        assert_eq!(
            codex_timeout_from_sources(&file_config, &SceneComposeEnv::default()),
            Err("Scene Codex timeout must be greater than 0 seconds".to_string())
        );
    }

    #[test]
    fn codex_failure_dialogue_classifies_rate_limit_and_auth() {
        let rate_limited = ComposeBackendResult {
            prompt: "say hi".to_string(),
            stdout: String::new(),
            stderr: "exceeded retry limit, last status: 429 Too Many Requests".to_string(),
            exit_code: Some(1),
            label: ComposeBackendLabel::Codex,
        };
        assert_eq!(rate_limited.failure_status(), "Codex unavailable");
        assert!(rate_limited
            .failure_dialogue(&rate_limited.stderr)
            .contains("rate limited"));

        let auth_blocked = ComposeBackendResult {
            prompt: "say hi".to_string(),
            stdout: String::new(),
            stderr: "failed to connect to websocket: HTTP error: 403 Forbidden".to_string(),
            exit_code: Some(1),
            label: ComposeBackendLabel::Codex,
        };
        assert_eq!(auth_blocked.failure_status(), "Codex unavailable");
        assert!(auth_blocked
            .failure_dialogue(&auth_blocked.stderr)
            .contains("could not connect"));
    }
}
