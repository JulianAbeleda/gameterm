#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-patch.sh COMMAND [OPTIONS]

Patch helper for GameTerm Scene Mode in-memory state updates.

Commands:
  apply                         Apply a patch to a scene with the Rust runtime.
  validate                      Validate a patch by applying it to a scene.
  write-inbox                   Atomically write a patch to a Scene Mode inbox.
  set-entity-status             Create a patch for one entity's flags/metadata.

Options for apply and validate:
  --scene PATH                  Scene file. Required.
  --patch PATH                  Patch file. Required.

Options for write-inbox:
  --inbox PATH                  GAMETERM_SCENE_PATCH_FILE path. Required.
  --patch PATH                  Patch file to copy. Required.

Options for set-entity-status:
  --output PATH                 Patch output path. Required.
  --entity-id ID                Entity id to update. Required.
  --status TEXT                 Runtime status string. Required.
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
    --status)
      status_text="$2"
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

create_entity_status_patch() {
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

  mkdir -p "$(dirname "${output_path}")"
  jq -n \
    --arg entity_id "${entity_id}" \
    --arg status "${status_text}" \
    --argjson flags "${flags_json}" \
    --argjson metadata "${metadata_json}" \
    '{
      scene_patch_version: 1,
      updates: [{
        entity_id: $entity_id,
        state_flags: $flags,
        metadata: $metadata
      }],
      status: $status
    }' | write_json "${output_path}"
  jq empty "${output_path}"
  echo "Wrote Scene Mode patch: ${output_path}"
}

cd "${repo_root}"

case "${command}" in
  apply)
    apply_patch_to_scene
    ;;
  validate)
    apply_patch_to_scene >/dev/null
    echo "Scene Mode patch is valid: ${patch_path}"
    ;;
  write-inbox)
    write_inbox
    ;;
  set-entity-status)
    create_entity_status_patch
    ;;
  *)
    echo "unknown command: ${command}" >&2
    usage >&2
    exit 2
    ;;
esac
