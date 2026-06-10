//! Local Scene composer slash commands.
//!
//! Slash commands are composer-local actions that never spawn the Codex
//! backend and never cost tokens. Parsing is pure so the event loop only
//! performs the resolved side effect.

/// A parsed composer slash command. `parse_scene_compose_command` returns
/// `None` for ordinary prompts, so the caller can fall through to the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SceneComposeCommand {
    /// `/model` with no argument reports the active model; with an argument it
    /// switches the runtime model override (alias already resolved).
    Model { name: Option<String> },
    /// `/clear` empties the compose transcript.
    Clear,
    /// `/help` lists the available commands.
    Help,
    /// A `/word` that is not a known command.
    Unknown(String),
}

/// Friendly model aliases. Anything not listed is passed through literally so
/// Codex validates it and surfaces rejection through the normal failure path.
const MODEL_ALIASES: &[(&str, &str)] = &[("spark", "gpt-5.3-codex-spark")];

pub(super) fn resolve_model_alias(name: &str) -> String {
    let trimmed = name.trim();
    for (alias, full) in MODEL_ALIASES {
        if trimmed.eq_ignore_ascii_case(alias) {
            return (*full).to_string();
        }
    }
    trimmed.to_string()
}

/// Parse a submitted composer buffer. Returns `None` when the input is not a
/// slash command and should be sent to the backend as a normal prompt.
pub(super) fn parse_scene_compose_command(input: &str) -> Option<SceneComposeCommand> {
    let trimmed = input.trim();
    let rest = trimmed.strip_prefix('/')?;
    // A bare "/" or "/ foo" is not a command; treat it as a normal prompt so a
    // user typing a path-like prompt is not hijacked.
    let mut parts = rest.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("");
    if name.is_empty() {
        return None;
    }
    let argument = parts.next().map(str::trim).filter(|arg| !arg.is_empty());
    let command = match name.to_ascii_lowercase().as_str() {
        "model" => SceneComposeCommand::Model {
            name: argument.map(resolve_model_alias),
        },
        "clear" => SceneComposeCommand::Clear,
        "help" => SceneComposeCommand::Help,
        _ => SceneComposeCommand::Unknown(name.to_string()),
    };
    Some(command)
}

/// The one-line `/help` summary.
pub(super) fn scene_compose_help_text() -> &'static str {
    "Commands: /model <name> switch model, /clear transcript, /help"
}

/// Footer hint shown while the composer draft is a slash command.
pub(super) fn scene_compose_slash_hint(active_model: &str) -> String {
    format!("slash: /model {active_model} · /clear · /help")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_prompt_is_not_a_command() {
        assert_eq!(parse_scene_compose_command("hello there"), None);
        assert_eq!(parse_scene_compose_command("~/path/to/file"), None);
        assert_eq!(parse_scene_compose_command("/"), None);
        assert_eq!(parse_scene_compose_command("/ leading space"), None);
    }

    #[test]
    fn model_command_parses_with_and_without_argument() {
        assert_eq!(
            parse_scene_compose_command("/model"),
            Some(SceneComposeCommand::Model { name: None })
        );
        assert_eq!(
            parse_scene_compose_command("/model spark"),
            Some(SceneComposeCommand::Model {
                name: Some("gpt-5.3-codex-spark".to_string())
            })
        );
    }

    #[test]
    fn model_alias_is_case_insensitive_and_passes_through_unknowns() {
        assert_eq!(resolve_model_alias("SPARK"), "gpt-5.3-codex-spark");
        assert_eq!(resolve_model_alias("gpt-5.5"), "gpt-5.5");
        assert_eq!(
            parse_scene_compose_command("/MODEL Spark"),
            Some(SceneComposeCommand::Model {
                name: Some("gpt-5.3-codex-spark".to_string())
            })
        );
    }

    #[test]
    fn clear_and_help_parse() {
        assert_eq!(
            parse_scene_compose_command("/clear"),
            Some(SceneComposeCommand::Clear)
        );
        assert_eq!(
            parse_scene_compose_command("/help"),
            Some(SceneComposeCommand::Help)
        );
    }

    #[test]
    fn unknown_command_is_reported() {
        assert_eq!(
            parse_scene_compose_command("/bogus stuff"),
            Some(SceneComposeCommand::Unknown("bogus".to_string()))
        );
    }
}
