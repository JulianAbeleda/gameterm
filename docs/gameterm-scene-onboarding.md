# GameTerm Scene Mode Onboarding

This is the shortest safe path from a repo to a usable Scene Mode workspace.

## Dry Run

Generate into `/tmp` first:

```sh
tmp="$(mktemp -d /tmp/gameterm-scene-onboarding.XXXXXX)"

ci/gameterm-scene-workspace.sh inspect --cwd .

ci/gameterm-scene-workspace.sh discover \
  --cwd . \
  --brief-output "${tmp}/task-brief.json" \
  --scene-output "${tmp}/workspace.json"

ci/gameterm-scene-author.sh validate "${tmp}/workspace.json"
ci/gameterm-scene-doctor.sh --scene "${tmp}/workspace.json"
```

Discovery does not run commands, start agents, submit prompts, or overwrite
config unless `--install` is used.

## Install

After the dry run validates:

```sh
ci/gameterm-scene-workspace.sh discover \
  --cwd . \
  --brief-output "${tmp}/task-brief.json" \
  --install
```

The install target is:

```text
${XDG_CONFIG_HOME:-~/.config}/gameterm/scenes/default.json
```

Install refuses to overwrite an existing scene unless `--force` is passed.

## Dogfood Workspace

Use the dogfood profile when the goal is to work on GameTerm through Scene Mode
itself:

```sh
tmp="$(mktemp -d /tmp/gameterm-scene-dogfood.XXXXXX)"

ci/gameterm-scene-workspace.sh dogfood \
  --cwd . \
  --brief-output "${tmp}/dogfood-task-brief.json" \
  --scene-output "${tmp}/dogfood-workspace.json" \
  --force

ci/gameterm-scene-author.sh validate "${tmp}/dogfood-workspace.json"
ci/gameterm-scene-doctor.sh --scene "${tmp}/dogfood-workspace.json"
```

To install it as the default Scene Mode workspace:

```sh
ci/gameterm-scene-workspace.sh dogfood --cwd . --install --force
```

The profile marks `dogfood_profile=true`, opens the roadmap/onboarding/smoke
docs, writes a task brief when requested or installed, and exposes explicit
confirmed choices for `ci/gameterm-scene-verify.sh --all`, `git status --short`,
and the focused dogfood smoke check.

## Active Pane

To generate a Scene Mode workspace from the active GameTerm pane, preview it
first:

```sh
ci/gameterm-scene-mux-context.sh discover \
  --scene-output "${tmp}/active-pane.json" \
  --force
ci/gameterm-scene-author.sh validate "${tmp}/active-pane.json"
```

Then install it as the default Scene Mode workspace:

```sh
ci/gameterm-scene-mux-context.sh discover --install --force
```

The helper uses active mux pane context when available. If mux context is
unavailable and a cwd-based fallback is acceptable, add `--allow-missing`. If
the active pane cwd is invalid, rerun from a valid pane or pass `--cwd PATH`;
the helper fails before writing the installed scene.

## Launch

Launch Scene Mode from GameTerm with the configured key assignment or command
surface for `ShowGameTermScene`.

## Smoke

Run deterministic verification:

```sh
ci/gameterm-scene-verify.sh --all
```

Run live smoke when a GUI session is available:

```sh
ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery
ci/gameterm-scene-smoke.sh --launch --scenario dogfood
```

## VN Assets

To verify the Rust-native VN asset path with repo-safe local fixtures:

```sh
cargo run -p gameterm-visual --example scene_vn_asset_intake -- \
  --catalog ci/fixtures/gameterm-scene/vn-demo-open-assets.json \
  --source-root ci/fixtures/gameterm-scene/vn-asset-source \
  --output-root /tmp/gameterm-vn-demo/assets/vn-demo \
  --sprite-manifest /tmp/gameterm-vn-demo/sprites.json \
  --attribution /tmp/gameterm-vn-demo/vn-demo-attribution.json \
  --bindings /tmp/gameterm-vn-demo/vn-demo-bindings.json \
  --base-manifest ci/fixtures/gameterm-scene/sprites.json \
  --force
```

The helper writes sprite IDs such as `vn.character.kiki.neutral`, attribution,
and bindings. It does not download or commit third-party assets.

If a downloaded VN character source is a layered PSD instead of ready-to-run
PNG sprites, flatten it into the expected local source-root layout first:

```sh
ci/gameterm-scene-vn-image-export.sh \
  --source ~/Downloads/character.psd \
  --output-source-root ~/.cache/gameterm-scene/vn-assets \
  --force

ci/gameterm-scene-smoke.sh \
  --launch \
  --scenario renderer-rows \
  --wait-before-capture 3 \
  --capture-timeout 8 \
  --output /tmp/gameterm-scene-renderer-rows.png
```

Downloaded PSDs and unclear-license art should stay local unless the source
explicitly allows redistribution and attribution/provenance is preserved.

## Recovery

If the generated scene is invalid, keep the installed scene untouched and rerun
the dry-run path into `/tmp`.

If an installed scene is bad:

```sh
mv "${XDG_CONFIG_HOME:-${HOME}/.config}/gameterm/scenes/default.json" \
  "${XDG_CONFIG_HOME:-${HOME}/.config}/gameterm/scenes/default.json.bak"

ci/gameterm-scene-author.sh install-fixture workspace-agent --force
ci/gameterm-scene-doctor.sh
```

Use `ci/gameterm-scene-author.sh validate` and `ci/gameterm-scene-doctor.sh`
before launching again.

Known macOS `unexpected cfg cargo-clippy` warnings and the existing
`phys_cell_idx` warning are build warning noise, not Scene Mode validation
failures.
