#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-init.sh [OPTIONS]

Creates editable GameTerm Scene Mode config from the bundled example scene.

Options:
  --config-home PATH  Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --force             Overwrite existing scene config files.
  --with-sprites      Also copy the example sprite manifest.
  -h, --help          Show this help.

By default this writes:
  ~/.config/gameterm/scenes/default.json

The sprite manifest is optional because its example image paths are starter
placeholders. Scene Mode can run without it by using bundled sprite defaults.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
force=0
with_sprites=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config-home)
      config_home="$2"
      shift 2
      ;;
    --force)
      force=1
      shift
      ;;
    --with-sprites)
      with_sprites=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

scene_dir="${config_home}/gameterm/scenes"
scene_path="${scene_dir}/default.json"
sprites_path="${scene_dir}/sprites.json"
example_scene="${repo_root}/docs/examples/gameterm-scene-default.json"
example_sprites="${repo_root}/docs/examples/gameterm-scene-sprites.json"

copy_file() {
  local source="$1"
  local target="$2"

  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi

  cp "${source}" "${target}"
  echo "Wrote ${target}"
}

mkdir -p "${scene_dir}"
copy_file "${example_scene}" "${scene_path}"

if [[ "${with_sprites}" -eq 1 ]]; then
  copy_file "${example_sprites}" "${sprites_path}"
fi

cat <<EOF

Scene authoring config is ready.

Edit:
  ${scene_path}

Then open Scene Mode with Ctrl+Shift+G and press r to reload after edits.
EOF

if [[ "${with_sprites}" -ne 1 ]]; then
  cat <<EOF

Optional sprite manifest:
  ci/gameterm-scene-init.sh --with-sprites
EOF
fi
