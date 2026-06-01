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
```

## VN Demo

To install the Rust VN demo path without local art:

```sh
ci/gameterm-scene-vn-demo.sh install --skip-assets --force
ci/gameterm-scene-vn-demo.sh doctor
```

To use approved local art, extract the asset sources outside the repo and pass
the root directory:

```sh
ci/gameterm-scene-vn-demo.sh install \
  --asset-source-root ~/Downloads/vn-assets \
  --force
ci/gameterm-scene-vn-demo.sh doctor
```

The helper validates generated output before install and refuses overwrites
unless `--force` is passed. It does not download or commit third-party assets.

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
