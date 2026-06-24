//! Persisted Voice settings for Scene Mode.
//!
//! gameterm's main config is user-authored Lua that the app only *reads*, so
//! menu toggles can't write back into it. Instead these runtime-toggleable
//! voice options live in a small app-managed JSON file (same pattern as the
//! scene story-state persistence): loaded at startup, rewritten when a Voice
//! menu toggle changes one. Environment variables remain the initial default
//! and an override; this file is the persisted source of truth across restarts.

use serde::{Deserialize, Serialize};
use std::path::Path;

const DEFAULT_VOICEVOX_PORT: u16 = 50021;

/// Which VOICEVOX engine endpoint Scene Mode talks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub(crate) enum VoicevoxHostMode {
    /// Talk to (and manage) a VOICEVOX engine on this machine at 127.0.0.1.
    /// This is the out-of-the-box default.
    #[default]
    Local,
    /// Talk to a remote VOICEVOX engine (e.g. a Mac mini on the LAN). Chosen
    /// explicitly, or seeded when the environment already points at a remote.
    Server,
}

/// Runtime-toggleable voice options surfaced in the Voice menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct VoiceSettings {
    /// Route TTS text through English -> Japanese translation before VOICEVOX.
    /// Default `false`: speak the original (English) text.
    pub translation_jpn: bool,
    /// Whether VOICEVOX runs locally on this machine or on a remote server.
    pub voicevox_host_mode: VoicevoxHostMode,
    /// `host` or `host:port` used when `voicevox_host_mode == Server`.
    pub voicevox_server: String,
    /// Command (argv) to launch a local VOICEVOX engine when host mode is
    /// `Local`. Empty means "assume an engine is already running at 127.0.0.1".
    pub voicevox_local_command: Vec<String>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            translation_jpn: false,
            voicevox_host_mode: VoicevoxHostMode::Local,
            voicevox_server: String::new(),
            voicevox_local_command: Vec::new(),
        }
    }
}

impl VoiceSettings {
    /// Load settings from `path`. A missing or unreadable/invalid file yields
    /// defaults — voice settings are best-effort and must never block startup.
    pub(crate) fn load_from_path(path: impl AsRef<Path>) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Persist settings to `path`, creating parent directories as needed.
    pub(crate) fn save_to_path(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
        std::fs::write(path, json)
    }

    /// Load from disk, or — on first run (no file) — seed from the environment
    /// so the VOICEVOX server the user already points at is captured and the
    /// local engine launch command is auto-detected.
    pub(crate) fn load_or_seed(path: impl AsRef<Path>) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_else(Self::from_env_defaults)
    }

    /// Seed defaults from the environment: VOICEVOX server from
    /// `GAMETERM_SCENE_TTS_VOICEVOX_HOST`/`VOICEVOX_HOST` (+ `_PORT`), and an
    /// auto-detected local engine command. Avoids hardcoding any address.
    ///
    /// Host mode defaults to `Local`. If the environment already points VOICEVOX
    /// at a non-loopback host (the user clearly has a server — e.g. a laptop
    /// pointing at a Mac mini), seed `Server` instead so that existing setup is
    /// preserved on first run.
    pub(crate) fn from_env_defaults() -> Self {
        let host_mode = if env_points_at_remote() {
            VoicevoxHostMode::Server
        } else {
            VoicevoxHostMode::Local
        };
        Self {
            voicevox_host_mode: host_mode,
            voicevox_server: env_voicevox_server(),
            voicevox_local_command: detect_local_voicevox_command(),
            ..Self::default()
        }
    }

    /// The `(host, port)` Scene Mode should send VOICEVOX requests to, based on
    /// the current host mode.
    pub(crate) fn resolve_endpoint(&self) -> (String, u16) {
        match self.voicevox_host_mode {
            VoicevoxHostMode::Local => ("127.0.0.1".to_string(), self.port()),
            VoicevoxHostMode::Server => {
                split_host_port(&self.voicevox_server).unwrap_or_else(default_endpoint)
            }
        }
    }

    fn port(&self) -> u16 {
        split_host_port(&self.voicevox_server)
            .map(|(_, port)| port)
            .unwrap_or(DEFAULT_VOICEVOX_PORT)
    }
}

fn default_endpoint() -> (String, u16) {
    ("127.0.0.1".to_string(), DEFAULT_VOICEVOX_PORT)
}

fn env_first(keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.trim().is_empty())
}

/// True when the environment explicitly points VOICEVOX at a non-loopback host.
fn env_points_at_remote() -> bool {
    match env_first(&["GAMETERM_SCENE_TTS_VOICEVOX_HOST", "VOICEVOX_HOST"]) {
        Some(host) => {
            let host = host.trim();
            !(host.is_empty() || host == "127.0.0.1" || host == "localhost" || host == "::1")
        }
        None => false,
    }
}

fn env_voicevox_server() -> String {
    let host = env_first(&["GAMETERM_SCENE_TTS_VOICEVOX_HOST", "VOICEVOX_HOST"])
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = env_first(&["GAMETERM_SCENE_TTS_VOICEVOX_PORT", "VOICEVOX_PORT"])
        .and_then(|value| value.trim().parse::<u16>().ok())
        .unwrap_or(DEFAULT_VOICEVOX_PORT);
    format!("{host}:{port}")
}

fn split_host_port(value: &str) -> Option<(String, u16)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some((host, port)) = value.rsplit_once(':') {
        if let Ok(port) = port.trim().parse::<u16>() {
            if !host.trim().is_empty() {
                return Some((host.trim().to_string(), port));
            }
        }
    }
    Some((value.to_string(), DEFAULT_VOICEVOX_PORT))
}

/// Best-effort detection of how to launch a local VOICEVOX engine. An empty
/// result means "assume an engine is already running at 127.0.0.1".
fn detect_local_voicevox_command() -> Vec<String> {
    // macOS: launching the app starts its bundled engine.
    if Path::new("/Applications/VOICEVOX.app").exists() {
        return vec!["open".to_string(), "-a".to_string(), "VOICEVOX".to_string()];
    }
    // A standalone engine on PATH.
    if which_on_path("voicevox_engine").is_some() {
        return vec!["voicevox_engine".to_string()];
    }
    Vec::new()
}

fn which_on_path(bin: &str) -> Option<std::path::PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(bin))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_english_and_local() {
        let s = VoiceSettings::default();
        assert!(!s.translation_jpn);
        assert_eq!(s.voicevox_host_mode, VoicevoxHostMode::Local);
    }

    #[test]
    fn missing_file_yields_defaults() {
        let s = VoiceSettings::load_from_path("/nonexistent/voice-settings.json");
        assert_eq!(s, VoiceSettings::default());
    }

    #[test]
    fn resolve_endpoint_switches_on_host_mode() {
        let mut s = VoiceSettings::default();
        s.voicevox_server = "192.168.1.50:50022".to_string();

        s.voicevox_host_mode = VoicevoxHostMode::Server;
        assert_eq!(s.resolve_endpoint(), ("192.168.1.50".to_string(), 50022));

        // Local keeps the configured port but forces loopback.
        s.voicevox_host_mode = VoicevoxHostMode::Local;
        assert_eq!(s.resolve_endpoint(), ("127.0.0.1".to_string(), 50022));
    }

    #[test]
    fn server_without_port_falls_back_to_default() {
        let mut s = VoiceSettings::default();
        s.voicevox_host_mode = VoicevoxHostMode::Server;
        s.voicevox_server = "macmini.local".to_string();
        assert_eq!(s.resolve_endpoint(), ("macmini.local".to_string(), 50021));
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("gameterm-voice-{}", std::process::id()));
        let path = dir.join("voice-settings.json");
        let mut s = VoiceSettings::default();
        s.translation_jpn = true;
        s.voicevox_host_mode = VoicevoxHostMode::Local;
        s.voicevox_server = "192.168.1.50:50021".to_string();
        s.save_to_path(&path).expect("save");
        let loaded = VoiceSettings::load_from_path(&path);
        assert_eq!(loaded, s);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
