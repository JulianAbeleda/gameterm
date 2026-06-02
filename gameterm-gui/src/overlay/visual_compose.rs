use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const COMPOSE_BACKEND_ENV: &str = "GAMETERM_SCENE_COMPOSE_BACKEND";
const COMPOSE_BACKEND_KIND_ENV: &str = "GAMETERM_SCENE_COMPOSE_BACKEND_KIND";
const COMPOSE_CODEX_BIN_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_BIN";
const COMPOSE_CODEX_WORKSPACE_ENV: &str = "GAMETERM_SCENE_COMPOSE_WORKSPACE";
const COMPOSE_CODEX_SANDBOX_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_SANDBOX";
const COMPOSE_CODEX_APPROVAL_ENV: &str = "GAMETERM_SCENE_COMPOSE_CODEX_APPROVAL";
const DEFAULT_CODEX_APPROVAL_POLICY: &str = "on-request";
const COMPOSE_BACKEND_TIMEOUT: Duration = Duration::from_secs(15);
const COMPOSE_BACKEND_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ComposeBackendRequest {
    pub(super) prompt: String,
    pub(super) scene_path: Option<String>,
    pub(super) pane_id: Option<usize>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ComposeBackendLabel {
    Compose,
    Codex,
}

impl ComposeBackendLabel {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ComposeBackendConfig {
    BuiltIn,
    Command(String),
    Codex(CodexComposeConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexComposeConfig {
    pub(super) program: String,
    pub(super) workspace: PathBuf,
    pub(super) sandbox: String,
    pub(super) approval: String,
    pub(super) json: bool,
}

pub(super) fn compose_running_status(prompt: &str) -> String {
    match compose_backend_config_from_env() {
        ComposeBackendConfig::Codex(_) => ComposeBackendLabel::Codex.running_status(prompt),
        ComposeBackendConfig::BuiltIn | ComposeBackendConfig::Command(_) => {
            ComposeBackendLabel::Compose.running_status(prompt)
        }
    }
}

pub(super) fn spawn_compose_backend(
    request: ComposeBackendRequest,
    tx: mpsc::Sender<ComposeBackendResult>,
) {
    thread::spawn(move || {
        let result = run_compose_backend(request);
        let _ = tx.send(result);
    });
}

fn run_compose_backend(request: ComposeBackendRequest) -> ComposeBackendResult {
    match compose_backend_config_from_env() {
        ComposeBackendConfig::BuiltIn => ComposeBackendResult {
            stdout: deterministic_compose_reply(&request.prompt),
            stderr: String::new(),
            exit_code: Some(0),
            prompt: request.prompt,
            label: ComposeBackendLabel::Compose,
        },
        ComposeBackendConfig::Command(command) => run_configured_compose_backend(request, command),
        ComposeBackendConfig::Codex(config) => run_codex_compose_backend(request, config),
    }
}

pub(super) fn compose_backend_config_from_env() -> ComposeBackendConfig {
    compose_backend_config(
        std::env::var(COMPOSE_BACKEND_KIND_ENV).ok().as_deref(),
        std::env::var(COMPOSE_BACKEND_ENV).ok().as_deref(),
        codex_compose_config_from_env(),
    )
}

pub(super) fn compose_backend_config(
    kind: Option<&str>,
    backend: Option<&str>,
    codex_config: CodexComposeConfig,
) -> ComposeBackendConfig {
    if kind
        .map(|value| value.trim().eq_ignore_ascii_case("codex"))
        .unwrap_or(false)
    {
        return ComposeBackendConfig::Codex(codex_config);
    }
    match backend.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) if value.eq_ignore_ascii_case("codex") => {
            ComposeBackendConfig::Codex(codex_config)
        }
        Some(value) => ComposeBackendConfig::Command(value.to_string()),
        None => ComposeBackendConfig::BuiltIn,
    }
}

fn codex_compose_config_from_env() -> CodexComposeConfig {
    CodexComposeConfig {
        program: std::env::var(COMPOSE_CODEX_BIN_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "codex".to_string()),
        workspace: compose_workspace_from_env(),
        sandbox: std::env::var(COMPOSE_CODEX_SANDBOX_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "read-only".to_string()),
        approval: std::env::var(COMPOSE_CODEX_APPROVAL_ENV)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CODEX_APPROVAL_POLICY.to_string()),
        json: true,
    }
}

fn compose_workspace_from_env() -> PathBuf {
    std::env::var(COMPOSE_CODEX_WORKSPACE_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn run_configured_compose_backend(
    request: ComposeBackendRequest,
    command: String,
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
        let _ = stdin.write_all(request.prompt.as_bytes());
    }

    match wait_for_child_output(child, COMPOSE_BACKEND_TIMEOUT) {
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
    config: CodexComposeConfig,
) -> ComposeBackendResult {
    let output_file = std::env::temp_dir().join(format!(
        "gameterm-scene-codex-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let argv = codex_compose_argv(&config, &output_file, &request.prompt);
    let Some((program, args)) = argv.split_first() else {
        return ComposeBackendResult {
            prompt: request.prompt,
            stdout: String::new(),
            stderr: "empty Codex backend command".to_string(),
            exit_code: None,
            label: ComposeBackendLabel::Codex,
        };
    };

    let result = run_codex_command(request.clone(), program, args, &output_file);
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

    match wait_for_child_output(child, COMPOSE_BACKEND_TIMEOUT) {
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
) -> Result<CollectedProcessOutput, CollectedProcessError> {
    let stdout = child.stdout.take().map(read_pipe);
    let stderr = child.stderr.take().map(read_pipe);
    let deadline = Instant::now() + timeout;

    loop {
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
    format!("I received your Scene Mode prompt: {}", prompt.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(prompt: &str) -> ComposeBackendRequest {
        ComposeBackendRequest {
            prompt: prompt.to_string(),
            scene_path: Some("scene.json".to_string()),
            pane_id: Some(7),
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
    fn compose_backend_config_selects_codex_explicitly() {
        let codex_config = CodexComposeConfig {
            program: "codex".to_string(),
            workspace: PathBuf::from("/workspace"),
            sandbox: "read-only".to_string(),
            approval: DEFAULT_CODEX_APPROVAL_POLICY.to_string(),
            json: true,
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
        );

        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "reply:short reply:hello:7\n");
    }

    #[test]
    fn run_configured_compose_backend_collects_large_stdout_without_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("large-stdout.sh");
        executable_script(
            &script,
            "#!/usr/bin/env sh\nyes x | head -n 20000\nprintf done\n",
        );

        let result = run_configured_compose_backend(request("large"), script.display().to_string());

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

        let result = run_configured_compose_backend(request("large"), script.display().to_string());

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

        let err = wait_for_child_output(child, Duration::from_millis(50)).unwrap_err();

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
            json: true,
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
            json: true,
        };
        let result = run_codex_compose_backend(request("look at roadmap"), config);

        assert_eq!(result.label, ComposeBackendLabel::Codex);
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.stdout, "Codex says: look at roadmap\n");
    }
}
