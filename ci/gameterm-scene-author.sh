#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-author.sh COMMAND [OPTIONS]

Authoring helper for GameTerm Scene Mode JSON files.

Commands:
  validate PATH                 Validate a scene file with the Rust scene parser.
  new-scene PATH                Create a minimal editable scene file.
  add-entity PATH               Add an entity to a scene file.
  add-choice PATH               Add a choice to a scene file.
  install-fixture NAME          Install a fixture into the Scene Mode config dir.
  list-fixtures                 List available authoring fixtures.

Common options:
  --config-home PATH            Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --force                       Overwrite existing scene config files.

Options for new-scene:
  --title TEXT                  Scene title. Default: New GameTerm Scene.
  --width N                     Scene width. Default: 12.
  --height N                    Scene height. Default: 7.

Options for add-entity:
  --id ID --kind KIND --label TEXT --x N --y N --sprite ID
  --flag FLAG                   Add one state flag. May be repeated.
  --metadata KEY=VALUE          Add one metadata pair. May be repeated.

Options for add-choice:
  --label TEXT
  --inspect
  --open-file PATH
  --navigate TARGET
  --run-argv JSON_ARRAY         Explicit argv array, for example:
                                '["cargo","check","-p","gameterm-visual"]'
  --cwd PATH                    Optional cwd for --run-argv.

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
title="New GameTerm Scene"
width=12
height=7
entity_id=""
entity_kind=""
entity_label=""
entity_x=""
entity_y=""
entity_sprite=""
choice_label=""
choice_kind=""
choice_payload=""
choice_cwd=""
flags=()
metadata=()

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
    --title)
      title="$2"
      shift 2
      ;;
    --width)
      width="$2"
      shift 2
      ;;
    --height)
      height="$2"
      shift 2
      ;;
    --id)
      entity_id="$2"
      shift 2
      ;;
    --kind)
      entity_kind="$2"
      shift 2
      ;;
    --label)
      if [[ "${command}" == "add-entity" ]]; then
        entity_label="$2"
      else
        choice_label="$2"
      fi
      shift 2
      ;;
    --x)
      entity_x="$2"
      shift 2
      ;;
    --y)
      entity_y="$2"
      shift 2
      ;;
    --sprite)
      entity_sprite="$2"
      shift 2
      ;;
    --flag)
      flags+=("$2")
      shift 2
      ;;
    --metadata)
      metadata+=("$2")
      shift 2
      ;;
    --inspect)
      choice_kind="Inspect"
      shift
      ;;
    --open-file)
      choice_kind="OpenFile"
      choice_payload="$2"
      shift 2
      ;;
    --navigate)
      choice_kind="Navigate"
      choice_payload="$2"
      shift 2
      ;;
    --run-argv)
      choice_kind="RunCommand"
      choice_payload="$2"
      shift 2
      ;;
    --cwd)
      choice_cwd="$2"
      shift 2
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

require_value() {
  local name="$1"
  local value="$2"
  if [[ -z "${value}" ]]; then
    echo "missing required ${name}" >&2
    usage >&2
    exit 2
  fi
}

validate_scene_file() {
  cargo run -q -p gameterm-visual --example scene_validate -- "$1"
}

write_json() {
  local target="$1"
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-author.XXXXXX)"
  cat >"${tmp}"
  mv "${tmp}" "${target}"
}

create_scene() {
  local target="$1"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    return 1
  fi

  mkdir -p "$(dirname "${target}")"
  jq -n \
    --arg title "${title}" \
    --argjson width "${width}" \
    --argjson height "${height}" \
    '{
      title: $title,
      background: "workspace-map",
      width: $width,
      height: $height,
      entities: [],
      dialogue_speaker: "GameTerm",
      dialogue: "Edit this scene, then press r in Scene Mode to reload.",
      choices: [{ label: "Inspect selected entity", kind: "Inspect" }]
    }' | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Wrote ${target}"
}

add_entity() {
  local target="$1"
  require_value "--id" "${entity_id}"
  require_value "--kind" "${entity_kind}"
  require_value "--label" "${entity_label}"
  require_value "--x" "${entity_x}"
  require_value "--y" "${entity_y}"
  require_value "--sprite" "${entity_sprite}"

  local flags_json metadata_json
  flags_json="$(printf '%s\n' "${flags[@]}" | jq -R -s 'split("\n")[:-1]')"
  metadata_json="$(printf '%s\n' "${metadata[@]}" | jq -R -s '
    split("\n")[:-1]
    | map(capture("(?<key>[^=]+)=(?<value>.*)") | [.key, .value])
  ')"

  jq \
    --arg id "${entity_id}" \
    --arg kind "${entity_kind}" \
    --arg label "${entity_label}" \
    --argjson x "${entity_x}" \
    --argjson y "${entity_y}" \
    --arg sprite "${entity_sprite}" \
    --argjson flags "${flags_json}" \
    --argjson metadata "${metadata_json}" \
    '.entities += [{
      id: $id,
      kind: $kind,
      label: $label,
      position: { x: $x, y: $y },
      sprite: $sprite,
      state_flags: $flags,
      metadata: $metadata
    }]' "${target}" | write_json "${target}"
  validate_scene_file "${target}" >/dev/null
  echo "Added entity ${entity_id} to ${target}"
}

add_choice() {
  local target="$1"
  require_value "--label" "${choice_label}"
  require_value "choice action" "${choice_kind}"

  case "${choice_kind}" in
    Inspect)
      jq --arg label "${choice_label}" \
        '.choices += [{ label: $label, kind: "Inspect" }]' \
        "${target}" | write_json "${target}"
      ;;
    OpenFile)
      require_value "--open-file" "${choice_payload}"
      jq --arg label "${choice_label}" --arg path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { OpenFile: { path: $path } } }]' \
        "${target}" | write_json "${target}"
      ;;
    Navigate)
      require_value "--navigate" "${choice_payload}"
      jq --arg label "${choice_label}" --arg target_path "${choice_payload}" \
        '.choices += [{ label: $label, kind: { Navigate: { target: $target_path } } }]' \
        "${target}" | write_json "${target}"
      ;;
    RunCommand)
      require_value "--run-argv" "${choice_payload}"
      if [[ -n "${choice_cwd}" ]]; then
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          --arg cwd "${choice_cwd}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv, cwd: $cwd } } }]' \
          "${target}" | write_json "${target}"
      else
        jq --arg label "${choice_label}" \
          --argjson argv "${choice_payload}" \
          '.choices += [{ label: $label, kind: { RunCommand: { argv: $argv } } }]' \
          "${target}" | write_json "${target}"
      fi
      ;;
  esac

  validate_scene_file "${target}" >/dev/null
  echo "Added choice ${choice_label} to ${target}"
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
    validate_scene_file "${positionals[0]}"
    ;;
  new-scene)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    create_scene "${positionals[0]}"
    ;;
  add-entity)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_entity "${positionals[0]}"
    ;;
  add-choice)
    if [[ "${#positionals[@]}" -ne 1 ]]; then
      usage >&2
      exit 2
    fi
    add_choice "${positionals[0]}"
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
