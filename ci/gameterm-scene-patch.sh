#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-patch.sh COMMAND [OPTIONS]

Patch helper for GameTerm Scene Mode in-memory state updates.

Commands:
  apply                         Apply a patch to a scene with the Rust runtime.
  export-scene                  Apply a patch and write a new scene JSON file.
  validate                      Validate a patch by applying it to a scene.
  write-inbox                   Atomically write a patch to a Scene Mode inbox.
  submit-mux                    Submit a patch through gameterm cli scene-patch.
  set-entity                    Create a patch for one entity's visual/runtime state.
  set-entity-status             Create a patch for one entity's flags/metadata.

Options for apply and validate:
  --scene PATH                  Scene file. Required.
  --patch PATH                  Patch file. Required.

Options for export-scene:
  --scene PATH                  Scene file. Required.
  --patch PATH                  Patch file. Required.
  --output PATH                 Patched scene output path. Required.
  --force                       Overwrite an existing output file.

Options for write-inbox:
  --inbox PATH                  GAMETERM_SCENE_PATCH_FILE path. Required.
  --patch PATH                  Patch file to copy. Required.

Options for submit-mux:
  --patch PATH                  Patch file to submit. Required.
  --target-pane-id ID           Target Scene Mode overlay pane. Optional.
  --source-pane-id ID           Source pane id. Optional.

Options for set-entity and set-entity-status:
  --output PATH                 Patch output path. Required.
  --entity-id ID                Entity id to update. Required.
  --status TEXT                 Runtime status string. Required.
  --label TEXT                  Entity label.
  --position X,Y                Entity grid position.
  --sprite ID                   Entity sprite id.
  --visible                     Mark entity visible.
  --hidden                      Mark entity hidden.
  --select-entity-id ID         Focus an entity after applying the patch.
  --flag FLAG                   State flag. May be repeated.
  --metadata KEY=VALUE          Metadata pair. May be repeated.
  --force                       Overwrite an existing output file.

Examples:
  ci/gameterm-scene-patch.sh apply \
    --scene ci/fixtures/gameterm-scene/default.json \
    --patch ci/fixtures/gameterm-scene/patch-status.json

  ci/gameterm-scene-patch.sh write-inbox \
    --inbox /tmp/gameterm-scene-patch.json \
    --patch ci/fixtures/gameterm-scene/patch-status.json

  ci/gameterm-scene-patch.sh export-scene \
    --scene ci/fixtures/gameterm-scene/default.json \
    --patch ci/fixtures/gameterm-scene/patch-status.json \
    --output /tmp/patched-scene.json --force

  ci/gameterm-scene-patch.sh submit-mux \
    --patch ci/fixtures/gameterm-scene/patch-status.json
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command="${1:-}"

if [[ -z "${command}" ]]; then
  usage >&2
  exit 2
fi
shift

scene_path=""
patch_path=""
inbox_path=""
output_path=""
entity_id=""
status_text=""
label_text=""
position_text=""
sprite_id=""
visible_state=""
selected_entity_id=""
target_pane_id=""
source_pane_id=""
force=0
flags=()
metadata=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scene)
      scene_path="$2"
      shift 2
      ;;
    --patch)
      patch_path="$2"
      shift 2
      ;;
    --inbox)
      inbox_path="$2"
      shift 2
      ;;
    --output)
      output_path="$2"
      shift 2
      ;;
    --entity-id)
      entity_id="$2"
      shift 2
      ;;
    --target-pane-id)
      target_pane_id="$2"
      shift 2
      ;;
    --source-pane-id)
      source_pane_id="$2"
      shift 2
      ;;
    --status)
      status_text="$2"
      shift 2
      ;;
    --label)
      label_text="$2"
      shift 2
      ;;
    --position)
      position_text="$2"
      shift 2
      ;;
    --sprite)
      sprite_id="$2"
      shift 2
      ;;
    --visible)
      visible_state="true"
      shift
      ;;
    --hidden)
      visible_state="false"
      shift
      ;;
    --select-entity-id)
      selected_entity_id="$2"
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
    --force)
      force=1
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

require_value() {
  local name="$1"
  local value="$2"
  if [[ -z "${value}" ]]; then
    echo "missing required ${name}" >&2
    usage >&2
    exit 2
  fi
}

write_json() {
  local target="$1"
  local tmp
  tmp="$(mktemp /tmp/gameterm-scene-patch.XXXXXX)"
  cat >"${tmp}"
  mv "${tmp}" "${target}"
}

apply_patch_to_scene() {
  require_value "--scene" "${scene_path}"
  require_value "--patch" "${patch_path}"
  cargo run -q -p gameterm-visual --example scene_patch_apply -- \
    "${scene_path}" \
    "${patch_path}"
}

write_inbox() {
  require_value "--inbox" "${inbox_path}"
  require_value "--patch" "${patch_path}"
  jq empty "${patch_path}"

  local inbox_dir tmp
  inbox_dir="$(dirname "${inbox_path}")"
  mkdir -p "${inbox_dir}"
  tmp="$(mktemp "${inbox_dir}/.gameterm-scene-patch.XXXXXX")"
  cp "${patch_path}" "${tmp}"
  mv "${tmp}" "${inbox_path}"
  echo "Wrote Scene Mode patch inbox: ${inbox_path}"
}

submit_mux() {
  require_value "--patch" "${patch_path}"
  jq empty "${patch_path}"

  local args
  args=(cli scene-patch --patch "${patch_path}")
  if [[ -n "${target_pane_id}" ]]; then
    args+=(--target-pane-id "${target_pane_id}")
  fi
  if [[ -n "${source_pane_id}" ]]; then
    args+=(--source-pane-id "${source_pane_id}")
  fi

  cargo run -q -p gameterm -- "${args[@]}"
}

export_scene() {
  require_value "--scene" "${scene_path}"
  require_value "--patch" "${patch_path}"
  require_value "--output" "${output_path}"
  if [[ -e "${output_path}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${output_path} already exists.

Rerun with --force to overwrite it.
EOF
    exit 1
  fi
  cargo run -q -p gameterm-visual --example scene_patch_export -- \
    "${scene_path}" \
    "${patch_path}" \
    "${output_path}"
}

create_entity_patch() {
  require_value "--output" "${output_path}"
  require_value "--entity-id" "${entity_id}"
  require_value "--status" "${status_text}"

  if [[ -e "${output_path}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${output_path} already exists.

Rerun with --force to overwrite it.
EOF
    exit 1
  fi

  local flags_json metadata_json
  flags_json="$(printf '%s\n' "${flags[@]}" | jq -R -s 'split("\n")[:-1]')"
  metadata_json="$(printf '%s\n' "${metadata[@]}" | jq -R -s '
    split("\n")[:-1]
    | map(capture("(?<key>[^=]+)=(?<value>.*)") | [.key, .value])
  ')"
  position_json="null"
  if [[ -n "${position_text}" ]]; then
    if [[ ! "${position_text}" =~ ^[0-9]+,[0-9]+$ ]]; then
      echo "--position must use X,Y with non-negative integers" >&2
      exit 2
    fi
    position_json="$(jq -n --arg position "${position_text}" '
      ($position | split(",")) as $parts
      | {x: ($parts[0] | tonumber), y: ($parts[1] | tonumber)}
    ')"
  fi
  visible_json="null"
  if [[ -n "${visible_state}" ]]; then
    visible_json="${visible_state}"
  fi

  mkdir -p "$(dirname "${output_path}")"
  jq -n \
    --arg entity_id "${entity_id}" \
    --arg status "${status_text}" \
    --arg label "${label_text}" \
    --arg sprite "${sprite_id}" \
    --arg selected_entity_id "${selected_entity_id}" \
    --argjson position "${position_json}" \
    --argjson visible "${visible_json}" \
    --argjson flags "${flags_json}" \
    --argjson metadata "${metadata_json}" \
    '{
      scene_patch_version: 1,
      updates: [{
        entity_id: $entity_id,
        label: (if $label == "" then null else $label end),
        position: $position,
        sprite: (if $sprite == "" then null else $sprite end),
        visible: $visible,
        state_flags: $flags,
        metadata: $metadata
      } | with_entries(select(.value != null))],
      selected_entity_id: (if $selected_entity_id == "" then null else $selected_entity_id end),
      status: $status
    } | with_entries(select(.value != null))' | write_json "${output_path}"
  jq empty "${output_path}"
  echo "Wrote Scene Mode patch: ${output_path}"
}

cd "${repo_root}"

case "${command}" in
  apply)
    apply_patch_to_scene
    ;;
  export-scene)
    export_scene
    ;;
  validate)
    apply_patch_to_scene >/dev/null
    echo "Scene Mode patch is valid: ${patch_path}"
    ;;
  write-inbox)
    write_inbox
    ;;
  submit-mux)
    submit_mux
    ;;
  set-entity)
    create_entity_patch
    ;;
  set-entity-status)
    create_entity_patch
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
