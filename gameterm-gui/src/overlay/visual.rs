use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_visual::{
    truncate_to_screen, SceneRuntime, VisualInput, VisualMode, VisualModeOutcome, VisualScene,
};
use mux::termwiztermtab::TermWizTerminal;
use std::path::PathBuf;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

pub fn show_visual_scene_overlay(mut term: TermWizTerminal) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Scene".to_string())])?;

    let scene_path = default_scene_path();
    let mut load_error = None;
    let mut runtime = match load_scene_runtime(&scene_path) {
        Ok(runtime) => {
            render_runtime(&mut term, &runtime)?;
            Some(runtime)
        }
        Err(err) => {
            let error = err.to_string();
            render_error(&mut term, &scene_path, &error)?;
            load_error.replace(error);
            None
        }
    };

    while let Some(input) = term.poll_input(None)? {
        match input {
            InputEvent::Key(KeyEvent { key, .. }) => {
                if let Some(runtime) = runtime.as_mut() {
                    if runtime.handle_input(visual_input_from_key(key)) == VisualModeOutcome::Exit {
                        break;
                    }
                    render_runtime(&mut term, runtime)?;
                }
            }
            InputEvent::Resized { .. } => {
                if let Some(runtime) = runtime.as_ref() {
                    render_runtime(&mut term, runtime)?;
                } else {
                    let error = load_error
                        .as_deref()
                        .unwrap_or("scene failed to load for an unknown reason");
                    render_error(&mut term, &scene_path, error)?;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn load_scene_runtime(scene_path: &PathBuf) -> anyhow::Result<SceneRuntime> {
    let scene = if scene_path.exists() {
        VisualScene::load_from_path(scene_path)?
    } else {
        VisualScene::demo()
    };
    Ok(SceneRuntime::new(scene)?)
}

fn default_scene_path() -> PathBuf {
    let config_home = config::CONFIG_DIRS
        .first()
        .cloned()
        .unwrap_or_else(|| config::HOME_DIR.join(".config").join("gameterm"));
    config_home.join("scenes").join("default.json")
}

fn visual_input_from_key(key: KeyCode) -> VisualInput {
    match key {
        KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => VisualInput::Close,
        KeyCode::Tab => VisualInput::ToggleDebug,
        KeyCode::Enter => VisualInput::Activate,
        KeyCode::RightArrow | KeyCode::DownArrow | KeyCode::Char('l') | KeyCode::Char('j') => {
            VisualInput::Next
        }
        KeyCode::LeftArrow | KeyCode::UpArrow | KeyCode::Char('h') | KeyCode::Char('k') => {
            VisualInput::Previous
        }
        _ => VisualInput::Other,
    }
}

fn render_runtime(term: &mut TermWizTerminal, runtime: &SceneRuntime) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    term.set_metadata(
        "gameterm_visual_snapshot",
        Value::String(serde_json::to_string(&runtime.render_snapshot())?),
    );
    let frame = runtime.render_text_frame(size.cols, size.rows);
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(frame),
    ])?;
    term.flush()?;
    Ok(())
}

fn render_error(
    term: &mut TermWizTerminal,
    scene_path: &PathBuf,
    error: &str,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let frame = format!(
        "GameTerm Scene Mode\r\n\
         Scene file failed to load.\r\n\r\n\
         Path: {}\r\n\
         Error: {}\r\n\r\n\
         Fix the scene JSON, or remove the file to use the built-in demo.\r\n\
         [esc/q: close]\r\n",
        scene_path.display(),
        error
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}
