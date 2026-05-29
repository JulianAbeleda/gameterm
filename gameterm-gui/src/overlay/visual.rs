use gameterm_term::color::ColorAttribute;
use gameterm_visual::{SceneRuntime, VisualScene};
use mux::termwiztermtab::TermWizTerminal;
use termwiz::input::{InputEvent, KeyCode, KeyEvent};
use termwiz::surface::Change;
use termwiz::terminal::Terminal;

pub fn show_visual_scene_overlay(mut term: TermWizTerminal) -> anyhow::Result<()> {
    term.no_grab_mouse_in_raw_mode();
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Scene".to_string())])?;

    let mut runtime = SceneRuntime::new(VisualScene::demo())?;
    render_runtime(&mut term, &runtime)?;

    while let Some(input) = term.poll_input(None)? {
        match input {
            InputEvent::Key(KeyEvent { key, .. }) => {
                if handle_key(&mut runtime, key) {
                    break;
                }
                render_runtime(&mut term, &runtime)?;
            }
            InputEvent::Resized { .. } => {
                render_runtime(&mut term, &runtime)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn handle_key(runtime: &mut SceneRuntime, key: KeyCode) -> bool {
    match key {
        KeyCode::Escape | KeyCode::Char('q') | KeyCode::Char('Q') => true,
        KeyCode::Tab => {
            runtime.toggle_debugger();
            false
        }
        KeyCode::Enter => {
            runtime.activate_choice();
            false
        }
        KeyCode::RightArrow | KeyCode::DownArrow | KeyCode::Char('l') | KeyCode::Char('j') => {
            runtime.select_next_entity();
            runtime.select_next_choice();
            false
        }
        KeyCode::LeftArrow | KeyCode::UpArrow | KeyCode::Char('h') | KeyCode::Char('k') => {
            runtime.select_prev_entity();
            runtime.select_prev_choice();
            false
        }
        _ => false,
    }
}

fn render_runtime(term: &mut TermWizTerminal, runtime: &SceneRuntime) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let frame = runtime.render_text_frame(size.cols, size.rows);
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::Text(frame),
    ])?;
    term.flush()?;
    Ok(())
}
