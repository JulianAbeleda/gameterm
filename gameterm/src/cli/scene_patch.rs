use anyhow::Context;
use clap::{Parser, ValueHint};
use gameterm_client::client::Client;
use mux::pane::PaneId;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Parser, Clone)]
pub struct ScenePatch {
    /// Target Scene Mode overlay pane. Defaults to the active Scene Mode overlay.
    #[arg(long)]
    target_pane_id: Option<PaneId>,

    /// Pane that produced the patch. Defaults to GAMETERM_PANE when set.
    #[arg(long)]
    source_pane_id: Option<PaneId>,

    /// Read the patch JSON from this file. If omitted, stdin is used.
    #[arg(long, value_hint = ValueHint::FilePath)]
    patch: Option<PathBuf>,
}

impl ScenePatch {
    pub async fn run(self, client: Client) -> anyhow::Result<()> {
        let patch_json = match self.patch {
            Some(path) => std::fs::read_to_string(&path)
                .with_context(|| format!("reading scene patch {}", path.display()))?,
            None => {
                let mut patch_json = String::new();
                std::io::stdin()
                    .read_to_string(&mut patch_json)
                    .context("reading scene patch from stdin")?;
                patch_json
            }
        };
        let source_pane_id = match self.source_pane_id {
            Some(pane_id) => Some(pane_id),
            None => client.resolve_pane_id(None).await.ok(),
        };

        let response = client
            .submit_gameterm_scene_patch(codec::SubmitGameTermScenePatch {
                patch_json,
                target_pane_id: self.target_pane_id,
                source_pane_id,
            })
            .await?;

        println!("{}", response.target_pane_id);
        Ok(())
    }
}
