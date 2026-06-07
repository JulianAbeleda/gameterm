use gameterm_visual::VisualInput;
use termwiz::input::{KeyCode, Modifiers};

pub(super) fn visual_input_from_key(key: KeyCode) -> VisualInput {
    match key {
        KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => VisualInput::Close,
        KeyCode::Char('r') | KeyCode::Char('R') => VisualInput::Reload,
        KeyCode::Tab => VisualInput::ToggleDebug,
        KeyCode::Enter => VisualInput::Activate,
        KeyCode::DownArrow | KeyCode::Char('j') | KeyCode::Char('J') => VisualInput::Next,
        KeyCode::UpArrow | KeyCode::Char('k') | KeyCode::Char('K') => VisualInput::Previous,
        KeyCode::RightArrow | KeyCode::Char('l') | KeyCode::Char('L') => VisualInput::Right,
        KeyCode::LeftArrow | KeyCode::Char('h') | KeyCode::Char('H') => VisualInput::Left,
        KeyCode::PageUp => VisualInput::ScrollDialogueUp,
        KeyCode::PageDown => VisualInput::ScrollDialogueDown,
        KeyCode::Backspace => VisualInput::Backspace,
        KeyCode::Char(c) => VisualInput::Char(c),
        _ => VisualInput::Other,
    }
}

pub(super) fn visual_input_resets_dialogue_scroll(input: VisualInput) -> bool {
    matches!(input, VisualInput::Activate)
}

pub(super) fn is_tts_toggle_key(key: KeyCode, modifiers: Modifiers) -> bool {
    matches!(key, KeyCode::Char('m') | KeyCode::Char('M')) && modifiers.contains(Modifiers::ALT)
}
