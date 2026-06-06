use gameterm_dynamic::Value;
use gameterm_term::color::ColorAttribute;
use gameterm_visual::{
    truncate_to_screen, vn_overlay_layout, vn_overlay_layout_with_overrides, SceneRuntime,
    VisualSceneSource, VisualSpriteManifestStatus, VisualView, VnOverlayRect,
};
use mux::termwiztermtab::TermWizTerminal;
use termwiz::surface::{Change, CursorVisibility, Position};
use termwiz::terminal::Terminal;

use super::visual_compose_dock::SceneComposeDock;
use super::visual_dialogue_scroll::SceneDialogueScrollback;
use super::visual_frame::{clip_text, replace_last_screen_line, replace_screen_line};
use super::visual_kiki_idle::{apply_kiki_idle_animation, current_kiki_idle_sprite};
use super::visual_voice_debug::SceneVoiceDebugState;

pub(super) fn render_runtime(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
) -> anyhow::Result<()> {
    render_runtime_with_compose(term, runtime, sprite_manifest, &SceneComposeDock::default())
}

pub(super) fn render_runtime_with_compose(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    compose_dock: &SceneComposeDock,
) -> anyhow::Result<()> {
    render_runtime_with_compose_and_scroll(
        term,
        runtime,
        sprite_manifest,
        compose_dock,
        &SceneDialogueScrollback::default(),
    )
}

pub(super) fn render_runtime_with_compose_and_scroll(
    term: &mut TermWizTerminal,
    runtime: &SceneRuntime,
    sprite_manifest: &VisualSpriteManifestStatus,
    compose_dock: &SceneComposeDock,
    dialogue_scroll: &SceneDialogueScrollback,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let mut snapshot = runtime.render_snapshot();
    apply_kiki_idle_animation(
        &mut snapshot,
        sprite_manifest,
        current_kiki_idle_sprite(sprite_manifest),
    );
    snapshot.overlay_cols = Some(size.cols);
    snapshot.overlay_rows = Some(size.rows);
    snapshot.vn_dialogue_scroll =
        Some(runtime.vn_dialogue_scroll_metrics(size.cols, size.rows, dialogue_scroll.offset));
    snapshot.vn_voice_hold_active = dialogue_scroll.voice_hold_active;
    term.set_metadata(
        "gameterm_visual_snapshot",
        Value::String(serde_json::to_string(&snapshot)?),
    );
    term.set_metadata(
        "gameterm_visual_sprites",
        Value::String(serde_json::to_string(sprite_manifest)?),
    );
    let mut frame = String::new();
    if snapshot.stage.is_empty() && !sprite_manifest.warnings.is_empty() {
        frame.push_str("Sprites: ");
        frame.push_str(&sprite_manifest.warnings.join("; "));
        frame.push_str("\r\n\r\n");
    }
    frame.push_str(
        &runtime.render_text_frame_with_dialogue_scroll_and_voice_hold(
            size.cols,
            size.rows,
            dialogue_scroll.offset,
            dialogue_scroll.voice_hold_active,
        ),
    );
    if !snapshot.stage.is_empty() && runtime.view() == VisualView::Scene {
        let layout = match snapshot.vn_layout_debug.as_ref() {
            Some(overrides) => vn_overlay_layout_with_overrides(
                size.cols,
                size.rows,
                &snapshot.dialogue_speaker,
                "Composer",
                overrides,
            ),
            None => vn_overlay_layout(size.cols, size.rows, &snapshot.dialogue_speaker, "Composer"),
        };
        if let Some(nameplate) = layout.composer_nameplate_text {
            frame = replace_screen_line(
                frame,
                size.cols,
                size.rows,
                nameplate.row.min(size.rows.saturating_sub(1)),
                &compose_dock.render_staged_nameplate_line(size.cols, nameplate),
            );
        }
        if let (Some(panel), Some(text_row)) = (layout.composer_panel, layout.composer_text_row) {
            let input_rect = VnOverlayRect {
                col: panel.col.saturating_add(layout.composer_text_inset_cols),
                row: text_row,
                width: panel
                    .width
                    .saturating_sub(layout.composer_text_inset_cols * 2),
                height: 1,
            };
            frame = replace_screen_line(
                frame,
                size.cols,
                size.rows,
                text_row,
                &compose_dock.render_staged_dock_line(size.cols, input_rect),
            );
        }
    } else {
        frame = replace_last_screen_line(
            frame,
            size.cols,
            size.rows,
            &compose_dock.render_line(size.cols),
        );
    }
    frame = apply_voice_debug_frame(
        frame,
        size.cols,
        size.rows,
        runtime,
        &dialogue_scroll.voice_debug,
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::CursorVisibility(CursorVisibility::Hidden),
        Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        },
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}

pub(super) fn apply_voice_debug_frame(
    mut frame: String,
    cols: usize,
    rows: usize,
    runtime: &SceneRuntime,
    voice_debug: &SceneVoiceDebugState,
) -> String {
    if runtime.view() != VisualView::VnLayoutDebugger {
        return frame;
    }
    let lines = voice_debug.render_lines();
    if lines.is_empty() {
        return frame;
    }
    let max_width = cols.min(96);
    let max_lines = rows.saturating_sub(1).min(lines.len());
    for (idx, line) in lines.iter().take(max_lines).enumerate() {
        frame = replace_screen_line(frame, cols, rows, idx, &clip_text(line, max_width));
    }
    frame
}

pub(super) fn render_error(
    term: &mut TermWizTerminal,
    source: &VisualSceneSource,
) -> anyhow::Result<()> {
    let size = term.get_screen_size()?;
    let frame = format!(
        "GameTerm Scene Mode\r\n\
         Scene file failed to load.\r\n\r\n\
         Path: {}\r\n\
         Load status: {}\r\n\
         Reload counter: {}\r\n\
         Error: {}\r\n\r\n\
         Fix the scene JSON, or remove the file to use the bundled default.\r\n\
         [r: reload] [esc/q: close]\r\n",
        source.scene_path,
        source.load_status.as_str(),
        source.reload_count,
        source
            .last_error
            .as_deref()
            .unwrap_or("scene failed to load for an unknown reason")
    );
    term.render(&[
        Change::ClearScreen(ColorAttribute::Default),
        Change::CursorVisibility(CursorVisibility::Hidden),
        Change::CursorPosition {
            x: Position::Absolute(0),
            y: Position::Absolute(0),
        },
        Change::Text(truncate_to_screen(frame, size.cols, size.rows)),
    ])?;
    term.flush()?;
    Ok(())
}
