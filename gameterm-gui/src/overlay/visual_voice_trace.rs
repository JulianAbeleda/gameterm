use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TRACE_ENV: &str = "GAMETERM_SCENE_TRACE";
const TRACE_FILE_ENV: &str = "GAMETERM_SCENE_TRACE_FILE";
const TRACE_FILE_NAME: &str = "scene-voice-trace.jsonl";

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct SceneVoiceTraceEvent {
    pub(crate) event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) turn_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) block_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) generation: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) speaker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) player_argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) player_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timing: Option<serde_json::Value>,
}

impl SceneVoiceTraceEvent {
    pub(crate) fn new(event: &'static str) -> Self {
        Self {
            event,
            ..Self::default()
        }
    }

    pub(crate) fn with_text(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.text_sha256 = Some(text_sha256(&text));
        self.text = Some(text);
        self
    }
}

pub(crate) fn scene_voice_trace_path() -> PathBuf {
    std::env::var(TRACE_FILE_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_trace_path)
}

pub(crate) fn trace_voice_event(event: SceneVoiceTraceEvent) {
    if trace_disabled() {
        return;
    }
    let path = scene_voice_trace_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let record = serde_json::json!({
        "ts_unix_ms": unix_ms(),
        "event": event.event,
        "turn_id": event.turn_id,
        "block_index": event.block_index,
        "generation": event.generation,
        "speaker": event.speaker,
        "text": event.text,
        "text_sha256": event.text_sha256,
        "status": event.status,
        "error": event.error,
        "output_path": event.output_path,
        "player_argv": event.player_argv,
        "player_pid": event.player_pid,
        "timing": event.timing,
    });
    let Ok(line) = serde_json::to_string(&record) else {
        return;
    };
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{line}");
    }
}

pub(crate) fn text_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn trace_disabled() -> bool {
    std::env::var(TRACE_ENV)
        .ok()
        .is_some_and(|value| matches!(value.trim(), "0" | "false" | "FALSE" | "off" | "OFF"))
}

fn default_trace_path() -> PathBuf {
    config::DATA_DIR.join(TRACE_FILE_NAME)
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_voice_trace_writes_jsonl_to_overridden_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");
        std::env::set_var(TRACE_FILE_ENV, &path);
        std::env::remove_var(TRACE_ENV);

        trace_voice_event(
            SceneVoiceTraceEvent::new("test_event")
                .with_text("Yes.")
                .tap(|event| {
                    event.turn_id = Some(7);
                    event.block_index = Some(1);
                    event.generation = Some(3);
                }),
        );

        let line = std::fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(value["event"], "test_event");
        assert_eq!(value["turn_id"], 7);
        assert_eq!(value["text"], "Yes.");
        assert_eq!(value["text_sha256"], text_sha256("Yes."));

        std::env::remove_var(TRACE_FILE_ENV);
    }

    trait Tap: Sized {
        fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
            f(&mut self);
            self
        }
    }

    impl<T> Tap for T {}
}
