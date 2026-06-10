//! Scene Mode voice configuration helpers.
//!
//! GameTerm opens straight into Scene Mode on startup (see
//! `TermWindow::show_boot_menu_once`), which presents the cozy boot screen and
//! its main menu. The legacy plain-text launcher boot menu was removed; these
//! helpers remain because Scene Mode is launched with voice configured by
//! default.

use super::visual_stt::SceneSttConfig;
use super::visual_tts::SceneTtsConfig;

pub(crate) fn voicevox_scene_tts_config() -> Result<SceneTtsConfig, String> {
    Ok(SceneTtsConfig::voicevox_default())
}

pub(crate) fn voice_scene_stt_config() -> Result<SceneSttConfig, String> {
    Ok(SceneSttConfig::whisper_voice_compose_default())
}

#[cfg(test)]
mod tests {
    #[test]
    fn voicevox_scene_choice_builds_tts_config() {
        let config = super::voicevox_scene_tts_config().unwrap();

        assert!(config.is_voicevox_backend());
    }

    #[test]
    fn voice_scene_choice_builds_stt_config() {
        let config = super::voice_scene_stt_config().unwrap();

        assert!(config.is_whisper_backend());
        assert!(config.auto_submits());
    }
}
