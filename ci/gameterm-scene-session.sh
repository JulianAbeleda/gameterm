#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ci/gameterm-scene-session.sh save --scene SCENE --output SESSION [--workspace-root PATH] [--force]
  ci/gameterm-scene-session.sh restore --scene SCENE --session SESSION --output STATE [--force]
  ci/gameterm-scene-session.sh validate --session SESSION
  ci/gameterm-scene-session.sh inspect --session SESSION

Explicit helper for Scene Mode workspace session state. It never rewrites the
source scene JSON.
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
session_path=""
output_path=""
workspace_root=""
force=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --scene)
      scene_path="$2"
      shift 2
      ;;
    --session)
      session_path="$2"
      shift 2
      ;;
    --output)
      output_path="$2"
      shift 2
      ;;
    --workspace-root)
      workspace_root="$2"
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
    echo "${name} is required" >&2
    usage >&2
    exit 2
  fi
}

write_output_file() {
  local source="$1"
  local target="$2"
  if [[ -e "${target}" && "${force}" -ne 1 ]]; then
    cat >&2 <<EOF
${target} already exists.

Rerun with --force to overwrite it.
EOF
    exit 1
  fi
  mkdir -p "$(dirname "${target}")"
  local tmp
  tmp="$(mktemp "${target}.XXXXXX")"
  cp "${source}" "${tmp}"
  mv "${tmp}" "${target}"
  echo "Wrote ${target}"
}

validate_session_json() {
  local path="$1"
  jq -e '
    .workspace_session_version == 1
    and (.scene_path | type == "string" and length > 0)
    and (.workspace_root | type == "string")
    and (.saved_at | type == "string" and length > 0)
    and (.story_state | type == "object")
    and .story_state.story_state_version == 1
  ' "${path}" >/dev/null
  local story_tmp
  story_tmp="$(mktemp /tmp/gameterm-scene-session-story.XXXXXX)"
  jq '.story_state' "${path}" >"${story_tmp}"
  "${repo_root}/ci/gameterm-scene-story.sh" validate "${story_tmp}" >/dev/null
  rm -f "${story_tmp}"
}

run_save() {
  require_value "--scene" "${scene_path}"
  require_value "--output" "${output_path}"
  if [[ ! -f "${scene_path}" ]]; then
    echo "scene file does not exist: ${scene_path}" >&2
    exit 2
  fi
  if [[ -z "${workspace_root}" ]]; then
    if git -C "$(dirname "${scene_path}")" rev-parse --show-toplevel >/tmp/gameterm-scene-session-root.out 2>/dev/null; then
      workspace_root="$(cat /tmp/gameterm-scene-session-root.out)"
    else
      workspace_root="$(cd "$(dirname "${scene_path}")" && pwd -P)"
    fi
  fi

  local story_tmp session_tmp saved_at
  story_tmp="$(mktemp /tmp/gameterm-scene-session-story.XXXXXX)"
  session_tmp="$(mktemp /tmp/gameterm-scene-session.XXXXXX)"
  "${repo_root}/ci/gameterm-scene-story.sh" export "${scene_path}" "${story_tmp}" >/dev/null
  saved_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  jq -n \
    --arg scene_path "${scene_path}" \
    --arg workspace_root "${workspace_root}" \
    --arg saved_at "${saved_at}" \
    --slurpfile story_state "${story_tmp}" \
    '{
      workspace_session_version: 1,
      scene_path: $scene_path,
      workspace_root: $workspace_root,
      saved_at: $saved_at,
      story_state: $story_state[0]
    }' >"${session_tmp}"
  validate_session_json "${session_tmp}"
  write_output_file "${session_tmp}" "${output_path}"
  rm -f "${story_tmp}" "${session_tmp}"
}

run_restore() {
  require_value "--scene" "${scene_path}"
  require_value "--session" "${session_path}"
  require_value "--output" "${output_path}"
  if [[ ! -f "${scene_path}" ]]; then
    echo "scene file does not exist: ${scene_path}" >&2
    exit 2
  fi
  if [[ ! -f "${session_path}" ]]; then
    echo "session file does not exist: ${session_path}" >&2
    exit 2
  fi
  validate_session_json "${session_path}"

  local story_tmp restored_tmp
  story_tmp="$(mktemp /tmp/gameterm-scene-session-story.XXXXXX)"
  restored_tmp="$(mktemp /tmp/gameterm-scene-session-restored.XXXXXX)"
  jq '.story_state' "${session_path}" >"${story_tmp}"
  "${repo_root}/ci/gameterm-scene-story.sh" \
    import \
    "${scene_path}" \
    "${story_tmp}" \
    "${restored_tmp}" >/dev/null
  write_output_file "${restored_tmp}" "${output_path}"
  rm -f "${story_tmp}" "${restored_tmp}"
}

run_validate() {
  require_value "--session" "${session_path}"
  validate_session_json "${session_path}"
  echo "Scene Mode workspace session is valid: ${session_path}"
}

run_inspect() {
  require_value "--session" "${session_path}"
  validate_session_json "${session_path}"
  jq -r '
    "workspace_session_version=\(.workspace_session_version)",
    "scene_path=\(.scene_path)",
    "workspace_root=\(.workspace_root)",
    "saved_at=\(.saved_at)",
    "variables=\(.story_state.variables | length)",
    "inventory=\(.story_state.rpg.inventory // [] | length)",
    "stats=\(.story_state.rpg.stats // [] | length)",
    "quests=\(.story_state.rpg.quests // [] | length)",
    "relationships=\(.story_state.rpg.relationships // [] | length)"
  ' "${session_path}"
}

case "${command}" in
  save)
    run_save
    ;;
  restore)
    run_restore
    ;;
  validate)
    run_validate
    ;;
  inspect)
    run_inspect
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
