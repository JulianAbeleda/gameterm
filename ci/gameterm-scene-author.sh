#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-author.sh COMMAND [OPTIONS]

Authoring helper for GameTerm Scene Mode JSON files.

Commands:
  validate PATH                 Validate a scene file with the Rust scene parser.
  install-fixture NAME          Install a fixture into the Scene Mode config dir.
  list-fixtures                 List available authoring fixtures.

Options for install-fixture:
  --config-home PATH            Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --force                       Overwrite existing scene config files.

Fixtures:
  basic, navigate, invalid, sprites, missing-sprite
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
command="${1:-}"

if [[ -z "${command}" ]]; then
  usage >&2
  exit 2
fi
shift

config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
force=0
positionals=()

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
    -h|--help)
      usage
      exit 0
      ;;
    --*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      positionals+=("$1")
      shift
      ;;
  esac
done

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

install_fixture() {
  local fixture="$1"
  local scene_dir="${config_home}/gameterm/scenes"
  mkdir -p "${scene_dir}"

  case "${fixture}" in
    basic)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      ;;
    navigate)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/memory.json" "${scene_dir}/memory.json"
      ;;
    invalid)
      copy_file "${fixture_root}/invalid.json" "${scene_dir}/default.json"
      ;;
    sprites)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites.json" "${scene_dir}/sprites.json"
      ;;
    missing-sprite)
      copy_file "${fixture_root}/default.json" "${scene_dir}/default.json"
      copy_file "${fixture_root}/sprites-missing.json" "${scene_dir}/sprites.json"
      ;;
    *)
      echo "unknown fixture: ${fixture}" >&2
      usage >&2
      exit 2
      ;;
  esac

  echo
  echo "Installed ${fixture} fixture into ${scene_dir}"
}

case "${command}" in
  validate)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    cargo run -q -p gameterm-visual --example scene_validate -- "${positionals[0]}"
    ;;
  install-fixture)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    install_fixture "${positionals[0]}"
    ;;
  list-fixtures)
    if [[ "${#positionals[@]}" -ne 0 ]]; then
      usage >&2
      exit 2
    fi
    printf '%s\n' basic navigate invalid sprites missing-sprite
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
