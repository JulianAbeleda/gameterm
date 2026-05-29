#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-doctor.sh [OPTIONS]

Checks Scene Mode scene and sprite authoring health.

Options:
  --scene PATH          Scene file to check. Default: config default.json.
  --sprites PATH        Sprite manifest to check. Default: config sprites.json.
  --config-home PATH    Use PATH instead of XDG_CONFIG_HOME or ~/.config.
  --strict              Treat warnings as failures.
  -h, --help            Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
config_home="${XDG_CONFIG_HOME:-${HOME}/.config}"
scene_path=""
sprites_path=""
strict=0
warnings=0
errors=0
scene_valid=0
sprite_manifest_valid=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scene)
      scene_path="$2"
      shift 2
      ;;
    --sprites)
      sprites_path="$2"
      shift 2
      ;;
    --config-home)
      config_home="$2"
      shift 2
      ;;
    --strict)
      strict=1
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

if [[ -z "${scene_path}" ]]; then
  scene_path="${config_home}/gameterm/scenes/default.json"
fi
if [[ -z "${sprites_path}" ]]; then
  sprites_path="${config_home}/gameterm/scenes/sprites.json"
fi

ok() {
  echo "OK: $*"
}

warn() {
  warnings=$((warnings + 1))
  echo "WARN: $*"
}

suggest() {
  echo "SUGGEST: $*"
}

error() {
  errors=$((errors + 1))
  echo "ERROR: $*"
}

resolve_path() {
  local base="$1"
  local path="$2"
  if [[ "${path}" = /* ]]; then
    printf '%s\n' "${path}"
  else
    printf '%s\n' "${base}/${path}"
  fi
}

scene_dir_for() {
  dirname "$1"
}

validate_scene() {
  if [[ ! -f "${scene_path}" ]]; then
    warn "scene file missing at ${scene_path}; Scene Mode will use the bundled default"
    suggest "create one with: ci/gameterm-scene-init.sh"
    return
  fi

  if cargo run -q -p gameterm-visual --example scene_validate -- "${scene_path}" >/tmp/gameterm-scene-doctor-scene.out 2>/tmp/gameterm-scene-doctor-scene.err; then
    ok "scene file is valid: ${scene_path}"
    cat /tmp/gameterm-scene-doctor-scene.out
    scene_valid=1
  else
    error "scene file is invalid: ${scene_path}"
    cat /tmp/gameterm-scene-doctor-scene.err >&2
    suggest "fix the reported scene schema issue, then rerun: ci/gameterm-scene-author.sh validate ${scene_path}"
  fi
}

check_navigate_targets() {
  if [[ "${scene_valid}" -ne 1 ]]; then
    return
  fi

  local scene_dir
  scene_dir="$(scene_dir_for "${scene_path}")"
  while IFS= read -r target; do
    [[ -z "${target}" ]] && continue
    local resolved
    resolved="$(resolve_path "${scene_dir}" "${target}")"
    if [[ -f "${resolved}" ]]; then
      ok "Navigate target exists: ${target}"
    else
      warn "Navigate target missing: ${target} -> ${resolved}"
      suggest "create ${resolved} or update this Navigate target in ${scene_path}"
    fi
  done < <(jq -r '.choices[]? | select(.kind.Navigate?) | .kind.Navigate.target' "${scene_path}")
}

check_open_file_targets() {
  if [[ "${scene_valid}" -ne 1 ]]; then
    return
  fi

  while IFS= read -r target; do
    [[ -z "${target}" ]] && continue
    local resolved
    resolved="$(resolve_path "${repo_root}" "${target}")"
    if [[ -f "${resolved}" ]]; then
      ok "OpenFile target exists: ${target}"
    else
      warn "OpenFile target missing from repo root: ${target} -> ${resolved}"
      suggest "update the OpenFile path or create ${resolved}"
    fi
  done < <(jq -r '.choices[]? | select(.kind.OpenFile?) | .kind.OpenFile.path' "${scene_path}")
}

check_run_command_actions() {
  if [[ "${scene_valid}" -ne 1 ]]; then
    return
  fi

  local index=0
  while IFS=$'\t' read -r label target cwd argv_len; do
    [[ -z "${label}" && -z "${target}" && -z "${cwd}" && -z "${argv_len}" ]] && continue
    if [[ "${cwd}" == "__GAMETERM_EMPTY__" ]]; then
      cwd=""
    fi
    index=$((index + 1))
    case "${target}" in
      tab|split_right|split_down)
        ok "RunCommand target is valid: ${label} -> ${target}"
        ;;
      *)
        error "RunCommand target is invalid: ${label} -> ${target}"
        suggest "set target to one of: tab, split_right, split_down"
        ;;
    esac

    if [[ "${argv_len}" == "0" ]]; then
      error "RunCommand argv is empty: ${label}"
      suggest "set an explicit argv array, for example: [\"true\"]"
    else
      ok "RunCommand argv is explicit: ${label} (${argv_len} arg(s))"
    fi

    if [[ -n "${cwd}" ]]; then
      local resolved
      resolved="$(resolve_path "${repo_root}" "${cwd}")"
      if [[ -d "${resolved}" ]]; then
        ok "RunCommand cwd exists: ${label} -> ${resolved}"
      else
        warn "RunCommand cwd missing or not a directory: ${label} -> ${resolved}"
        suggest "create ${resolved}, remove cwd, or update cwd for ${label}"
      fi
    fi
  done < <(
    jq -r '
      .choices[]?
      | select(.kind.RunCommand?)
      | [
          .label,
          (.kind.RunCommand.target // "tab"),
          (.kind.RunCommand.cwd // "__GAMETERM_EMPTY__"),
          ((.kind.RunCommand.argv // []) | length)
        ]
      | @tsv
    ' "${scene_path}"
  )
}

validate_sprite_manifest() {
  if [[ ! -f "${sprites_path}" ]]; then
    warn "sprite manifest missing at ${sprites_path}; Scene Mode will use bundled sprites and placeholders"
    suggest "create starter config with: ci/gameterm-scene-init.sh --with-sprites"
    return
  fi

  if jq -e '.sprites | type == "array"' "${sprites_path}" >/dev/null; then
    ok "sprite manifest JSON shape is valid: ${sprites_path}"
    sprite_manifest_valid=1
  else
    error "sprite manifest must contain a sprites array: ${sprites_path}"
    suggest "use the shape from docs/examples/gameterm-scene-sprites.json"
    return
  fi

  local empty_ids duplicate_ids empty_paths
  empty_ids="$(jq -r '[.sprites[]? | select((.id // "" | gsub("[[:space:]]"; "")) == "")] | length' "${sprites_path}")"
  duplicate_ids="$(jq -r '[.sprites[]?.id] | group_by(.) | map(select(length > 1) | .[0]) | .[]?' "${sprites_path}")"
  empty_paths="$(jq -r '[.sprites[]? | select((.path // "" | gsub("[[:space:]]"; "")) == "")] | length' "${sprites_path}")"

  if [[ "${empty_ids}" != "0" ]]; then
    error "sprite manifest has ${empty_ids} empty sprite id(s)"
    suggest "set every sprite entry id to a non-empty scene sprite id"
  fi
  if [[ -n "${duplicate_ids}" ]]; then
    while IFS= read -r id; do
      error "sprite manifest has duplicate sprite id: ${id}"
      suggest "keep one entry for sprite id ${id}"
    done <<<"${duplicate_ids}"
  fi
  if [[ "${empty_paths}" != "0" ]]; then
    error "sprite manifest has ${empty_paths} empty sprite path(s)"
    suggest "set every sprite entry path to a PNG file path"
  fi

  local manifest_dir
  manifest_dir="$(scene_dir_for "${sprites_path}")"
  while IFS=$'\t' read -r id path; do
    [[ -z "${id}" && -z "${path}" ]] && continue
    local resolved
    resolved="$(resolve_path "${manifest_dir}" "${path}")"
    if [[ -f "${resolved}" ]]; then
      ok "sprite asset exists: ${id} -> ${resolved}"
    else
      warn "sprite asset missing: ${id} -> ${resolved}"
      suggest "create ${resolved} or update sprite id ${id} in ${sprites_path}"
    fi
  done < <(jq -r '.sprites[]? | [.id, .path] | @tsv' "${sprites_path}")
}

check_scene_sprite_coverage() {
  if [[ "${scene_valid}" -ne 1 || "${sprite_manifest_valid}" -ne 1 ]]; then
    return
  fi

  local scene_sprite_ids manifest_sprite_ids missing_ids
  scene_sprite_ids="$(mktemp /tmp/gameterm-scene-doctor-scene-sprites.XXXXXX)"
  manifest_sprite_ids="$(mktemp /tmp/gameterm-scene-doctor-manifest-sprites.XXXXXX)"
  jq -r '[.background, (.entities[]?.sprite)] | .[]' "${scene_path}" | sort -u >"${scene_sprite_ids}"
  jq -r '.sprites[]?.id' "${sprites_path}" | sort -u >"${manifest_sprite_ids}"
  missing_ids="$(comm -23 "${scene_sprite_ids}" "${manifest_sprite_ids}")"

  if [[ -z "${missing_ids}" ]]; then
    ok "sprite manifest covers all scene sprite ids"
  else
    while IFS= read -r id; do
      warn "scene sprite id has no manifest entry: ${id}"
      suggest "add to ${sprites_path}: {\"id\":\"${id}\",\"path\":\"sprites/${id}.png\"}"
    done <<<"${missing_ids}"
  fi

  rm -f "${scene_sprite_ids}" "${manifest_sprite_ids}"
}

echo "Scene Mode doctor"
echo "Scene: ${scene_path}"
echo "Sprites: ${sprites_path}"
echo

validate_scene
check_navigate_targets
check_open_file_targets
check_run_command_actions
validate_sprite_manifest
check_scene_sprite_coverage

echo
echo "Doctor summary: ${errors} error(s), ${warnings} warning(s)"

if [[ "${errors}" -gt 0 ]]; then
  exit 1
fi
if [[ "${strict}" -eq 1 && "${warnings}" -gt 0 ]]; then
  exit 1
fi
