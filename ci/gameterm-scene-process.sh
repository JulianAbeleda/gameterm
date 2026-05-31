#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-process.sh [OPTIONS] -- COMMAND [ARG...]

Run a process and emit Scene Mode patches before and after it runs.

Options:
  --entity-id ID                Entity id to update. Required.
  --patch PATH                  Patch output path. Required unless --inbox is set.
  --inbox PATH                  Also write each patch to a Scene Mode inbox.
  --submit-mux                  Submit each patch through gameterm cli scene-patch.
  --target-pane-id ID           Target Scene Mode overlay pane for --submit-mux.
  --source-pane-id ID           Source pane id for --submit-mux.
  --label TEXT                  Entity label to set in the final patch.
  --sprite ID                   Entity sprite to set in the final patch.
  --select                      Focus the entity when applying patches.
  -h, --help                    Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
entity_id=""
patch_path=""
inbox_path=""
submit_mux=0
target_pane_id=""
source_pane_id=""
label_text=""
sprite_id=""
select_entity=0
command_argv=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --entity-id)
      entity_id="$2"
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
    --submit-mux)
      submit_mux=1
      shift
      ;;
    --target-pane-id)
      target_pane_id="$2"
      shift 2
      ;;
    --source-pane-id)
      source_pane_id="$2"
      shift 2
      ;;
    --label)
      label_text="$2"
      shift 2
      ;;
    --sprite)
      sprite_id="$2"
      shift 2
      ;;
    --select)
      select_entity=1
      shift
      ;;
    --)
      shift
      command_argv=("$@")
      break
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

if [[ -z "${entity_id}" ]]; then
  echo "missing required --entity-id" >&2
  usage >&2
  exit 2
fi
if [[ -z "${patch_path}" && -z "${inbox_path}" ]]; then
  echo "missing required --patch or --inbox" >&2
  usage >&2
  exit 2
fi
if [[ "${#command_argv[@]}" -eq 0 ]]; then
  echo "missing command after --" >&2
  usage >&2
  exit 2
fi
if [[ -z "${patch_path}" ]]; then
  patch_path="$(mktemp /tmp/gameterm-scene-process.XXXXXX.json)"
fi

write_patch() {
  local status="$1"
  local flag="$2"
  local exit_code="$3"
  local command_text="$4"
  local process_phase="$5"
  local process_message="$6"
  shift 6
  local extra_args=("$@")

  local args=(
    set-entity
    --output "${patch_path}"
    --entity-id "${entity_id}"
    --status "${status}"
    --flag "${flag}"
    --metadata "exit_code=${exit_code}"
    --metadata "command=${command_text}"
    --process-phase "${process_phase}"
    --process-command "${command_text}"
    --process-exit-code "${exit_code}"
    --process-message "${process_message}"
    --force
  )
  if [[ -n "${label_text}" ]]; then
    args+=(--label "${label_text}")
  fi
  if [[ -n "${sprite_id}" ]]; then
    args+=(--sprite "${sprite_id}")
  fi
  if [[ "${select_entity}" -eq 1 ]]; then
    args+=(--select-entity-id "${entity_id}")
  fi
  args+=("${extra_args[@]}")

  "${repo_root}/ci/gameterm-scene-patch.sh" "${args[@]}" >/dev/null

  if [[ -n "${inbox_path}" ]]; then
    "${repo_root}/ci/gameterm-scene-patch.sh" write-inbox \
      --inbox "${inbox_path}" \
      --patch "${patch_path}" >/dev/null
  fi
  if [[ "${submit_mux}" -eq 1 ]]; then
    local submit_args=(submit-mux --patch "${patch_path}")
    if [[ -n "${target_pane_id}" ]]; then
      submit_args+=(--target-pane-id "${target_pane_id}")
    fi
    if [[ -n "${source_pane_id}" ]]; then
      submit_args+=(--source-pane-id "${source_pane_id}")
    fi
    "${repo_root}/ci/gameterm-scene-patch.sh" "${submit_args[@]}" >/dev/null
  fi
}

command_text="${command_argv[*]}"
write_patch \
  "Process running: ${command_text}" \
  running \
  0 \
  "${command_text}" \
  running \
  "Process running" \
  --visible

set +e
"${command_argv[@]}"
rc=$?
set -e

if [[ "${rc}" -eq 0 ]]; then
  write_patch \
    "Process succeeded: ${command_text}" \
    succeeded \
    "${rc}" \
    "${command_text}" \
    succeeded \
    "Process succeeded" \
    --visible
else
  write_patch \
    "Process failed (${rc}): ${command_text}" \
    failed \
    "${rc}" \
    "${command_text}" \
    failed \
    "Process failed" \
    --visible
fi

exit "${rc}"
