//! Cozy-game navigation shell: the boot screen, the main menu, and the
//! in-scene Tab-cycle mode screens. These screens own keyboard input directly
//! (the compose dock is bypassed for shell views) and render through the
//! text-frame path. Pixel-art skinning is a later pass.

use super::SceneRuntime;
use crate::vn_text::truncate_to_screen;
use crate::{VisualInput, VisualModeOutcome, VisualView};

const MAIN_MENU_ITEMS: &[&str] = &["Continue", "New Session", "Settings", "Native Terminal"];

const MODE_TABS: &[(VisualView, &str)] = &[
    (VisualView::CharacterSelect, "Character Select"),
    (VisualView::StageSelect, "Stage Select"),
    (VisualView::SettingMode, "Setting Mode"),
];

// Placeholder list content per mode screen. Real catalogs and swap behavior
// are wired after assets land.
const CHARACTER_ITEMS: &[&str] = &["Kiki", "Guide", "(add character)"];
const STAGE_ITEMS: &[&str] = &["Classroom", "Hallway", "(add stage)"];
const SETTING_ITEMS: &[&str] = &["Voice", "Text speed", "Open debug menu"];

impl SceneRuntime {
    /// Enter the boot screen. The overlay calls this on open so Scene Mode
    /// always starts at "press start".
    pub fn enter_boot(&mut self) {
        self.view = VisualView::Boot;
        self.shell_cursor = 0;
        self.bump_generation();
    }

    /// Enter the scene proper, leaving any shell screen.
    fn enter_scene(&mut self) {
        self.view = VisualView::Scene;
        self.shell_cursor = 0;
        self.bump_generation();
    }

    /// Enter the Tab-cycle mode screens, starting at Character Select.
    pub(super) fn enter_mode_cycle(&mut self) {
        self.view = VisualView::CharacterSelect;
        self.shell_cursor = 0;
        self.bump_generation();
    }

    fn shell_items(&self) -> &'static [&'static str] {
        match self.view {
            VisualView::MainMenu => MAIN_MENU_ITEMS,
            VisualView::CharacterSelect => CHARACTER_ITEMS,
            VisualView::StageSelect => STAGE_ITEMS,
            VisualView::SettingMode => SETTING_ITEMS,
            _ => &[],
        }
    }

    fn move_shell_cursor(&mut self, forward: bool) {
        let len = self.shell_items().len();
        if len == 0 {
            return;
        }
        self.shell_cursor = if forward {
            (self.shell_cursor + 1) % len
        } else {
            (self.shell_cursor + len - 1) % len
        };
        self.bump_generation();
    }

    pub(super) fn handle_shell_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        match self.view {
            VisualView::Boot => self.handle_boot_input(input),
            VisualView::MainMenu => self.handle_main_menu_input(input),
            _ if self.view.is_mode_cycle() => self.handle_mode_cycle_input(input),
            _ => VisualModeOutcome::Continue,
        }
    }

    fn handle_boot_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        match input {
            VisualInput::Activate => {
                self.view = VisualView::MainMenu;
                self.shell_cursor = 0;
                self.bump_generation();
                VisualModeOutcome::Continue
            }
            VisualInput::Close => VisualModeOutcome::Exit,
            _ => VisualModeOutcome::Continue,
        }
    }

    fn handle_main_menu_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        match input {
            VisualInput::Next => {
                self.move_shell_cursor(true);
                VisualModeOutcome::Continue
            }
            VisualInput::Previous => {
                self.move_shell_cursor(false);
                VisualModeOutcome::Continue
            }
            VisualInput::Activate => {
                match MAIN_MENU_ITEMS.get(self.shell_cursor).copied() {
                    // Continue and New Session both enter the scene in this
                    // pass; the resume/reset difference is wired later.
                    Some("Continue") | Some("New Session") => self.enter_scene(),
                    Some("Settings") => self.open_layout_debugger(),
                    // Native Terminal closes the Scene overlay, dropping back to
                    // the plain terminal underneath.
                    Some("Native Terminal") => return VisualModeOutcome::Exit,
                    _ => {}
                }
                VisualModeOutcome::Continue
            }
            VisualInput::Close => {
                self.view = VisualView::Boot;
                self.shell_cursor = 0;
                self.bump_generation();
                VisualModeOutcome::Continue
            }
            _ => VisualModeOutcome::Continue,
        }
    }

    fn handle_mode_cycle_input(&mut self, input: VisualInput) -> VisualModeOutcome {
        match input {
            VisualInput::ToggleDebug => {
                let next = (self.mode_tab_index() + 1) % MODE_TABS.len();
                self.view = MODE_TABS[next].0;
                self.shell_cursor = 0;
                self.bump_generation();
                VisualModeOutcome::Continue
            }
            VisualInput::Next => {
                self.move_shell_cursor(true);
                VisualModeOutcome::Continue
            }
            VisualInput::Previous => {
                self.move_shell_cursor(false);
                VisualModeOutcome::Continue
            }
            VisualInput::Activate => {
                let label = self.shell_items().get(self.shell_cursor).copied();
                if let Some(label) = label {
                    if self.view == VisualView::SettingMode && label == "Open debug menu" {
                        self.open_layout_debugger();
                    } else {
                        let mode = self.mode_tab_label();
                        self.status = format!("{mode}: {label} (not yet wired)");
                        self.bump_generation();
                    }
                }
                VisualModeOutcome::Continue
            }
            VisualInput::Close => {
                self.enter_scene();
                VisualModeOutcome::Continue
            }
            _ => VisualModeOutcome::Continue,
        }
    }

    fn mode_tab_index(&self) -> usize {
        MODE_TABS
            .iter()
            .position(|(view, _)| *view == self.view)
            .unwrap_or(0)
    }

    fn mode_tab_label(&self) -> &'static str {
        MODE_TABS[self.mode_tab_index()].1
    }

    pub(super) fn render_shell_screen(&self, cols: usize, rows: usize) -> String {
        let out = match self.view {
            VisualView::Boot => self.render_boot_screen(),
            VisualView::MainMenu => self.render_main_menu(),
            _ if self.view.is_mode_cycle() => self.render_mode_cycle(),
            _ => String::new(),
        };
        truncate_to_screen(out, cols, rows)
    }

    fn render_boot_screen(&self) -> String {
        let mut out = String::new();
        out.push_str("\r\n\r\n");
        out.push_str(&center_line(&self.scene.title));
        out.push_str("\r\n\r\n");
        out.push_str(&center_line("G A M E T E R M"));
        out.push_str("\r\n\r\n\r\n");
        out.push_str(&center_line("Press Enter to Start"));
        out.push_str("\r\n\r\n");
        out.push_str(&center_line("[esc: close]"));
        out.push_str("\r\n");
        out
    }

    fn render_main_menu(&self) -> String {
        let mut out = String::new();
        out.push_str(&center_line("- Main Menu -"));
        out.push_str("\r\n\r\n");
        for (idx, item) in MAIN_MENU_ITEMS.iter().enumerate() {
            let marker = if idx == self.shell_cursor { ">" } else { " " };
            out.push_str(&center_line(&format!("{marker} {item}")));
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
        out.push_str(&center_line(
            "[up/down: select] [enter: choose] [esc: back]",
        ));
        out.push_str("\r\n");
        out
    }

    fn render_mode_cycle(&self) -> String {
        let mut out = String::new();
        let tabs = MODE_TABS
            .iter()
            .map(|(view, label)| {
                if *view == self.view {
                    format!("[{label}]")
                } else {
                    format!(" {label} ")
                }
            })
            .collect::<Vec<_>>()
            .join("   ");
        out.push_str(&center_line(&tabs));
        out.push_str("\r\n\r\n");
        for (idx, item) in self.shell_items().iter().enumerate() {
            let marker = if idx == self.shell_cursor { ">" } else { " " };
            out.push_str(&center_line(&format!("{marker} {item}")));
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
        out.push_str(&center_line(
            "[tab: next mode] [up/down: select] [esc: back to scene]",
        ));
        out.push_str(&format!("\r\n\r\nStatus: {}\r\n", self.status));
        out
    }
}

const SHELL_WIDTH: usize = 72;

fn center_line(text: &str) -> String {
    let width = text.chars().count();
    if width >= SHELL_WIDTH {
        return text.to_string();
    }
    let pad = (SHELL_WIDTH - width) / 2;
    format!("{}{}", " ".repeat(pad), text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VisualMode, VisualScene};

    fn runtime() -> SceneRuntime {
        SceneRuntime::new(VisualScene::demo()).unwrap()
    }

    #[test]
    fn boot_advances_to_menu_then_into_scene() {
        let mut rt = runtime();
        rt.enter_boot();
        assert_eq!(rt.view(), VisualView::Boot);
        assert!(rt.render_text_frame(80, 24).contains("Press Enter to Start"));

        rt.handle_shell_input(VisualInput::Activate);
        assert_eq!(rt.view(), VisualView::MainMenu);

        // Default cursor is Continue, which enters the scene.
        rt.handle_shell_input(VisualInput::Activate);
        assert_eq!(rt.view(), VisualView::Scene);
    }

    #[test]
    fn boot_close_exits_overlay() {
        let mut rt = runtime();
        rt.enter_boot();
        assert_eq!(rt.handle_shell_input(VisualInput::Close), VisualModeOutcome::Exit);
    }

    #[test]
    fn plain_backdrop_covers_shell_and_debug_views_only() {
        for view in [
            VisualView::Boot,
            VisualView::MainMenu,
            VisualView::CharacterSelect,
            VisualView::StageSelect,
            VisualView::SettingMode,
            VisualView::TileDebugger,
            VisualView::VnLayoutDebugger,
        ] {
            assert!(view.uses_plain_backdrop(), "{:?}", view);
        }
        assert!(!VisualView::Scene.uses_plain_backdrop());
        assert!(!VisualView::CommandSelection.uses_plain_backdrop());
    }

    #[test]
    fn main_menu_native_terminal_exits_overlay() {
        let mut rt = runtime();
        rt.enter_boot();
        rt.handle_shell_input(VisualInput::Activate); // -> MainMenu
        // Cursor up from Continue wraps to the last item, Native Terminal.
        rt.handle_shell_input(VisualInput::Previous);
        let frame = rt.render_text_frame(80, 24);
        assert!(frame.contains("Native Terminal"));
        assert_eq!(
            rt.handle_shell_input(VisualInput::Activate),
            VisualModeOutcome::Exit
        );
    }

    #[test]
    fn main_menu_cursor_wraps_and_routes_each_item() {
        let mut rt = runtime();
        rt.enter_boot();
        rt.handle_shell_input(VisualInput::Activate); // -> MainMenu

        // New Session also enters the scene.
        rt.handle_shell_input(VisualInput::Next); // New Session
        rt.handle_shell_input(VisualInput::Activate);
        assert_eq!(rt.view(), VisualView::Scene);

        // Settings (index 2) routes to the layout debugger.
        rt.enter_boot();
        rt.handle_shell_input(VisualInput::Activate); // MainMenu
        rt.handle_shell_input(VisualInput::Next); // New Session
        rt.handle_shell_input(VisualInput::Next); // Settings
        rt.handle_shell_input(VisualInput::Activate);
        assert_eq!(rt.view(), VisualView::VnLayoutDebugger);
    }

    #[test]
    fn scene_tab_enters_mode_cycle_and_tab_cycles_all_three() {
        let mut rt = runtime();
        assert_eq!(rt.view(), VisualView::Scene);
        rt.handle_input(VisualInput::ToggleDebug);
        assert_eq!(rt.view(), VisualView::CharacterSelect);
        rt.handle_shell_input(VisualInput::ToggleDebug);
        assert_eq!(rt.view(), VisualView::StageSelect);
        rt.handle_shell_input(VisualInput::ToggleDebug);
        assert_eq!(rt.view(), VisualView::SettingMode);
        rt.handle_shell_input(VisualInput::ToggleDebug);
        assert_eq!(rt.view(), VisualView::CharacterSelect);
    }

    #[test]
    fn mode_cycle_esc_returns_to_scene_and_list_moves() {
        let mut rt = runtime();
        rt.handle_input(VisualInput::ToggleDebug); // -> CharacterSelect
        rt.handle_shell_input(VisualInput::Next);
        assert_eq!(rt.shell_cursor, 1);
        let frame = rt.render_text_frame(80, 24);
        assert!(frame.contains("Character Select"));
        rt.handle_shell_input(VisualInput::Close);
        assert_eq!(rt.view(), VisualView::Scene);
    }
}
