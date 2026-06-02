use crate::termwindow::TermWindowNotif;
use config::keyassignment::KeyAssignment;
use mux::pane::PaneId;
use mux::termwiztermtab::TermWizTerminal;
use termwiz::cell::{AttributeChange, CellAttributes};
use termwiz::color::ColorAttribute;
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers, MouseButtons, MouseEvent};
use termwiz::surface::{Change, Position};
use termwiz::terminal::Terminal;
use window::WindowOps;

#[derive(Clone, Copy)]
enum BootChoice {
    Scene,
    Native,
}

impl BootChoice {
    fn label(self) -> &'static str {
        match self {
            BootChoice::Scene => "1. Scene Mode",
            BootChoice::Native => "2. Native Terminal Mode",
        }
    }

    fn assignment(self) -> Option<KeyAssignment> {
        match self {
            BootChoice::Scene => Some(KeyAssignment::ShowGameTermScene),
            BootChoice::Native => None,
        }
    }
}

struct BootMenuState {
    active_idx: usize,
    pane_id: PaneId,
    window: ::window::Window,
}

impl BootMenuState {
    fn render(&self, term: &mut TermWizTerminal) -> termwiz::Result<()> {
        let choices = [BootChoice::Scene, BootChoice::Native];
        let mut changes = vec![
            Change::ClearScreen(ColorAttribute::Default),
            Change::CursorPosition {
                x: Position::Absolute(0),
                y: Position::Absolute(0),
            },
            Change::Text("GameTerm Boot Menu\r\n\r\n".to_string()),
            Change::Text("Choose boot mode:\r\n\r\n".to_string()),
        ];

        for (idx, choice) in choices.iter().enumerate() {
            if idx == self.active_idx {
                changes.push(AttributeChange::Reverse(true).into());
            }
            changes.push(Change::Text(format!("  {}\r\n", choice.label())));
            if idx == self.active_idx {
                changes.push(AttributeChange::Reverse(false).into());
            }
            changes.push(Change::AllAttributes(CellAttributes::default()));
        }

        changes.push(Change::Text(
            "\r\nPress 1/2, Enter to choose, or Esc for native terminal.\r\n".to_string(),
        ));
        term.render(&changes)
    }

    fn choose(&self, choice: BootChoice) {
        if let Some(assignment) = choice.assignment() {
            self.window.notify(TermWindowNotif::PerformAssignment {
                pane_id: self.pane_id,
                assignment,
                tx: None,
            });
        }
    }

    fn run_loop(&mut self, term: &mut TermWizTerminal) -> anyhow::Result<()> {
        let choices = [BootChoice::Scene, BootChoice::Native];
        while let Ok(Some(event)) = term.poll_input(None) {
            match event {
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('1'),
                    ..
                }) => {
                    self.choose(BootChoice::Scene);
                    break;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('2'),
                    ..
                })
                | InputEvent::Key(KeyEvent {
                    key: KeyCode::Escape,
                    ..
                }) => break,
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Char('g' | 'G'),
                    modifiers,
                }) if modifiers.contains(Modifiers::CTRL)
                    && modifiers.contains(Modifiers::SHIFT) =>
                {
                    self.choose(BootChoice::Scene);
                    break;
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::UpArrow,
                    ..
                }) => {
                    self.active_idx = self.active_idx.saturating_sub(1);
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::DownArrow,
                    ..
                }) => {
                    self.active_idx = (self.active_idx + 1).min(choices.len() - 1);
                }
                InputEvent::Mouse(MouseEvent {
                    y, mouse_buttons, ..
                }) => {
                    if mouse_buttons == MouseButtons::LEFT {
                        let idx = y.saturating_sub(4) as usize;
                        if let Some(choice) = choices.get(idx).copied() {
                            self.choose(choice);
                            break;
                        }
                    }
                }
                InputEvent::Key(KeyEvent {
                    key: KeyCode::Enter,
                    ..
                }) => {
                    self.choose(choices[self.active_idx]);
                    break;
                }
                _ => {}
            }
            self.render(term)?;
        }

        Ok(())
    }
}

pub fn boot_menu(mut term: TermWizTerminal, window: ::window::Window) -> anyhow::Result<()> {
    let pane_id = term.pane_id().ok_or_else(|| {
        anyhow::anyhow!("GameTerm boot menu terminal is not attached to a mux pane")
    })?;
    let mut state = BootMenuState {
        active_idx: 0,
        pane_id,
        window,
    };
    term.set_raw_mode()?;
    term.render(&[Change::Title("GameTerm Boot Menu".to_string())])?;
    state.render(&mut term)?;
    state.run_loop(&mut term)
}

#[cfg(test)]
mod tests {
    use super::BootChoice;
    use config::keyassignment::KeyAssignment;

    #[test]
    fn boot_menu_scene_choice_routes_to_configured_scene_mode() {
        assert_eq!(BootChoice::Scene.label(), "1. Scene Mode");
        assert!(matches!(
            BootChoice::Scene.assignment(),
            Some(KeyAssignment::ShowGameTermScene)
        ));
    }

    #[test]
    fn boot_menu_native_choice_exits_without_overlay_assignment() {
        assert_eq!(BootChoice::Native.label(), "2. Native Terminal Mode");
        assert!(BootChoice::Native.assignment().is_none());
    }
}
