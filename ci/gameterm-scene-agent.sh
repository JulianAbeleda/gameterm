#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-agent.sh status [OPTIONS]

Emit a Scene Mode patch for an agent lifecycle phase.

Options:
  --entity-id ID                Entity id to update. Required.
  --phase PHASE                 idle, planning, running, waiting, blocked,
                                complete/completed, failed, or cancelled.
                                Required.
  --message TEXT                Agent status message.
  --command TEXT                Agent command/task label.
  --patch PATH                  Patch output path. Required unless --inbox is set.
  --inbox PATH                  Also write the patch to a Scene Mode inbox.
  --submit-mux                  Submit the patch through gameterm cli scene-patch.
  --target-pane-id ID           Target Scene Mode overlay pane for --submit-mux.
  --source-pane-id ID           Source pane id for --submit-mux.
  --label TEXT                  Entity label to set.
  --sprite ID                   Entity sprite to set.
  --select                      Focus the entity when applying the patch.
  -h, --help                    Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
command="${1:-}"

if [[ -z "${command}" ]]; then
  usage >&2
  exit 2
fi
shift

entity_id=""
agent_phase=""
message_text=""
command_text=""
patch_path=""
inbox_path=""
submit_mux=0
target_pane_id=""
source_pane_id=""
label_text=""
sprite_id=""
select_entity=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --entity-id)
      entity_id="$2"
      shift 2
      ;;
    --phase)
      agent_phase="$2"
      shift 2
      ;;
    --message)
      message_text="$2"
      shift 2
      ;;
    --command)
      command_text="$2"
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

if [[ "${command}" != "status" ]]; then
  echo "unknown command: ${command}" >&2
  usage >&2
  exit 2
fi
if [[ -z "${entity_id}" ]]; then
  echo "missing required --entity-id" >&2
  usage >&2
  exit 2
fi
if [[ -z "${agent_phase}" ]]; then
  echo "missing required --phase" >&2
  usage >&2
  exit 2
fi
if [[ -z "${patch_path}" && -z "${inbox_path}" ]]; then
  echo "missing required --patch or --inbox" >&2
  usage >&2
  exit 2
fi
if [[ -z "${patch_path}" ]]; then
  patch_path="$(mktemp /tmp/gameterm-scene-agent.XXXXXX.json)"
fi
if [[ -z "${command_text}" ]]; then
  command_text="agent:${agent_phase}"
fi

process_phase=""
exit_code_args=()
agent_phase_flag="${agent_phase}"
case "${agent_phase}" in
  idle)
    process_phase="queued"
    ;;
  planning)
    process_phase="queued"
    ;;
  running)
    process_phase="running"
    ;;
  waiting)
    process_phase="blocked"
    ;;
  blocked)
    process_phase="blocked"
    ;;
  complete|completed)
    process_phase="succeeded"
    agent_phase_flag="completed"
    exit_code_args=(--process-exit-code 0)
    ;;
  failed)
    process_phase="failed"
    exit_code_args=(--process-exit-code 1)
    ;;
  cancelled)
    process_phase="failed"
    exit_code_args=(--process-exit-code 130)
    ;;
  *)
    echo "--phase must be idle, planning, running, waiting, blocked, complete, completed, failed, or cancelled" >&2
    exit 2
    ;;
esac

status_text="Agent ${agent_phase}"
if [[ -n "${message_text}" ]]; then
  status_text="${status_text}: ${message_text}"
else
  message_text="${status_text}"
fi

args=(
  set-entity
  --output "${patch_path}"
  --entity-id "${entity_id}"
  --status "${status_text}"
  --flag agent
  --flag "agent_${agent_phase_flag}"
  --metadata "agent_phase=${agent_phase_flag}"
  --metadata "agent_command=${command_text}"
  --process-phase "${process_phase}"
  --process-command "${command_text}"
  --process-message "${message_text}"
  --force
)
if ((${#exit_code_args[@]} > 0)); then
  args+=("${exit_code_args[@]}")
fi
if [[ -n "${label_text}" ]]; then
  args+=(--label "${label_text}")
fi
if [[ -n "${sprite_id}" ]]; then
  args+=(--sprite "${sprite_id}")
fi
if [[ "${select_entity}" -eq 1 ]]; then
  args+=(--select-entity-id "${entity_id}")
fi

"${repo_root}/ci/gameterm-scene-patch.sh" "${args[@]}" >/dev/null

tmp_patch="$(mktemp /tmp/gameterm-scene-agent-state.XXXXXX.json)"
jq \
  --arg phase "${agent_phase_flag}" \
  --arg process_phase "${process_phase}" \
  '.variables += [
    {key: "agent_phase", value: {Text: $phase}},
    {key: "agent_process_phase", value: {Text: $process_phase}}
  ]' \
  "${patch_path}" >"${tmp_patch}"
mv "${tmp_patch}" "${patch_path}"

if [[ -n "${inbox_path}" ]]; then
  "${repo_root}/ci/gameterm-scene-patch.sh" write-inbox \
    --inbox "${inbox_path}" \
    --patch "${patch_path}" >/dev/null
fi
if [[ "${submit_mux}" -eq 1 ]]; then
  submit_args=(submit-mux --patch "${patch_path}")
  if [[ -n "${target_pane_id}" ]]; then
    submit_args+=(--target-pane-id "${target_pane_id}")
  fi
  if [[ -n "${source_pane_id}" ]]; then
    submit_args+=(--source-pane-id "${source_pane_id}")
  fi
  "${repo_root}/ci/gameterm-scene-patch.sh" "${submit_args[@]}" >/dev/null
fi

echo "Wrote Scene Mode agent patch: ${patch_path}"
